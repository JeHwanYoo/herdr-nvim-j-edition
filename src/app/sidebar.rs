//! The nvim sidebar in the Herdr app: a real pane of the tab, split off the
//! tab's whole tree so it runs full height beside every other pane. Its
//! terminal carries `herdr-nvim.sidebar=<tab>`, which is how a toggle finds it.

use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::{
    config::{self, SidebarPosition},
    host::Opened,
    maneuver::Ctx,
    state::TabId,
};

use super::{
    cli::{self, AppCtx},
    doc, host, raw_tab, rpc, tab_id,
};

pub(crate) const LABEL: &str = "herdr-nvim.sidebar";

/// The app splits only right or down; left/top fall back to them.
pub(crate) fn direction(position: SidebarPosition) -> &'static str {
    match position {
        SidebarPosition::Right | SidebarPosition::Left => "right",
        SidebarPosition::Bottom | SidebarPosition::Top => "down",
    }
}

/// The live sidebar terminals of `tab` (`<space>:<tab>`), or of every tab
/// (`None`): `(terminal, shown)`. One the mod parked (`terminal.park`: its
/// pane put away, nvim's screen kept for the next Files, sync.js) is live but
/// in no pane.
fn sidebars(tab: Option<&str>) -> Result<Vec<(String, bool)>> {
    let filter = match tab {
        Some(tab) => json!({"labels": {LABEL: raw_tab(tab)}}),
        None => json!({}),
    };
    let list = rpc::call("terminal.list", filter)?;
    let doc = doc::read()?;
    Ok(list
        .get("terminals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|t| t.get("state").and_then(Value::as_str) == Some("live"))
        .filter(|t| t.pointer(&format!("/labels/{LABEL}")).is_some())
        .filter_map(|t| t.get("address").and_then(Value::as_str))
        .map(|terminal| {
            let shown = doc
                .as_ref()
                .is_some_and(|doc| doc::pane_of(doc, terminal).is_some());
            (terminal.to_owned(), shown)
        })
        .collect())
}

/// The sidebar terminal `tab` (`<space>:<tab>`) shows, if any.
pub(crate) fn find(tab: &str) -> Result<Option<String>> {
    Ok(sidebars(Some(tab))?
        .into_iter()
        .find_map(|(terminal, shown)| shown.then_some(terminal)))
}

/// Ends the parked sidebar terminals of `tab` (every tab: `None`): before a
/// new one is made for it, and after an install (a parked one shows the nvim
/// from before; the next Files makes a new one, restarting a stale clean nvim).
/// The tab's nvim itself keeps running.
pub(crate) fn end_parked(tab: Option<&str>) -> Result<usize> {
    let parked: Vec<String> = sidebars(tab)?
        .into_iter()
        .filter_map(|(terminal, shown)| (!shown).then_some(terminal))
        .collect();
    for terminal in &parked {
        rpc::call("terminal.kill", json!({"terminal": terminal}))?;
    }
    Ok(parked.len())
}

/// Close the pane showing `terminal` (its program ends with it).
pub(crate) fn close(terminal: &str) -> Result<()> {
    match doc::read()?.and_then(|doc| doc::pane_of(&doc, terminal)) {
        Some((pane, _)) => cli::app_do("pane.close", &json!({"pane": pane})).map(drop),
        None => cli::app_do("terminal.kill", &json!({"terminal": terminal})).map(drop),
    }
}

/// Give `terminal`'s pane the keyboard. A dialog still on screen (the picker,
/// closing as its program exits) hands the keyboard back to the pane it was
/// opened over when it goes, so wait for it to go first (at most 3 s; each
/// look runs the app binary, so not more often than every 150 ms).
pub(crate) fn focus(terminal: &str) -> Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline
        && cli::app_status()
            .ok()
            .and_then(|status| status.get("overlays").cloned())
            .and_then(|overlays| overlays.as_array().cloned())
            .is_some_and(|overlays| overlays.iter().any(|o| o == "layout"))
    {
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    cli::app_do("pane.focus", &json!({"terminal": terminal})).map(drop)
}

/// Split the whole tab and run the sidebar in the new pane: `pane.split`
/// accepts any node of the tab, so splitting its root node puts the sidebar
/// beside all of its panes, full height. A preview showing in nvim's place
/// (a file opened in nvim while it shows) is swapped back instead: nvim takes
/// the preview's pane.
///
/// A stale daemon (an older runtime's) is restarted first, or kept with a
/// notice when it has unsaved work (`stale`).
pub(crate) fn open(ctx: &Ctx) -> Result<Opened> {
    let doc = doc::read()?;
    let folder = nvim_folder(ctx);
    let notice = super::stale::refresh_on_open(ctx.tab.as_str(), &folder).unwrap_or_else(|error| {
        eprintln!("herdr-nvim: could not check this tab's nvim for updates: {error:#}");
        None
    });
    let config = config::load();
    crate::daemon::ready_to_open(ctx.tab.as_str(), &config.sidebar)?;
    // One nvim screen per tab: a parked one (from before an install, or in
    // a tab whose panes changed since) goes.
    end_parked(Some(ctx.tab.as_str()))?;
    let preview = doc
        .as_ref()
        .and_then(|doc| doc::preview_of(doc, raw_tab(&ctx.tab)));
    let anchor = match &preview {
        Some((pane, _)) => pane.clone(),
        None => doc
            .as_ref()
            .and_then(|doc| doc::root_of(doc, raw_tab(&ctx.tab)))
            .with_context(|| format!("tab {} is not in the app's layout", raw_tab(&ctx.tab)))?,
    };
    // `tree = "nvim"` opens nvim's own explorer with it (`sidebar_ready`).
    let argv = vec![
        cli::self_command()?.to_string_lossy().into_owned(),
        "sidebar".to_owned(),
        "--cwd".to_owned(),
        folder.to_string_lossy().into_owned(),
    ];
    // In a preview's place: across, so nvim takes the preview's exact slot
    // (`swap_direction`); else beside the whole tab.
    let split = match preview {
        Some(_) => swap_direction(),
        None => direction(config.sidebar.position),
    };
    let made = cli::app_do(
        "pane.split",
        &json!({
            "direction": split,
            "pane": anchor,
            "argv": argv,
        }),
    )?;
    let terminal = made
        .get("terminal")
        .and_then(Value::as_str)
        .context("pane.split made no terminal")?
        .to_owned();
    // The sidebar labels itself too; doing it here as well means a toggle
    // right after this one already finds it.
    label(&terminal, raw_tab(&ctx.tab))?;
    if let Some((pane, _)) = preview {
        cli::app_do("pane.close", &json!({"pane": pane}))?;
    }
    Ok(Opened {
        pane: terminal,
        notice,
    })
}

/// Show `file` rendered (the `herdr-nvim` mod's preview role) where the tab's
/// nvim shows. In the place of nvim's pane or of a preview on screen (the
/// pane header's Preview, another file, an image opened any way): split
/// across that pane (`swap_direction`), so both nest in its slot, then close
/// it, and the new pane keeps its exact size. nvim keeps running (its pane
/// only attaches to the tab's daemon). With neither on screen (Files
/// reopening a preview it closed, an image with nothing open): beside the
/// whole tab, as `open` does for nvim. Every pane herdr-nvim makes is made
/// here or in `open`; the mod only closes them.
///
/// Prints `{"preview": pane}`, with `closed` (nvim's terminal) or
/// `closed_preview` (the old preview's pane) for the one it replaced.
pub(crate) fn open_preview(ctx: &Ctx, file: &str) -> Result<Value> {
    let config = config::load();
    let doc = doc::read()?.context("the app's layout is unknown")?;
    let raw = raw_tab(&ctx.tab);
    let nvim = find(&ctx.tab)?
        .and_then(|terminal| doc::pane_of(&doc, &terminal).map(|(pane, _)| (pane, terminal)));
    let old_preview = doc::preview_of(&doc, raw).map(|(pane, _)| pane);
    let (anchor, split) = match (&nvim, &old_preview) {
        (Some((pane, _)), _) | (None, Some(pane)) => (pane.clone(), swap_direction()),
        (None, None) => (
            doc::root_of(&doc, raw)
                .with_context(|| format!("tab {raw} is not in the app's layout"))?,
            direction(config.sidebar.position),
        ),
    };
    let made = cli::app_do(
        "pane.split",
        &json!({
            "direction": split,
            "pane": anchor,
            "mod": super::layout_patch::MOD,
            "props": {"preview": file, "tab": raw, "space": ctx.workspace, "cwd": ctx.cwd},
        }),
    )?;
    // The preview's pane (`true` when the app didn't name it).
    let pane = made.get("pane").cloned().filter(|p| !p.is_null());
    let mut out = json!({"preview": pane.unwrap_or(json!(true))});
    match (nvim, old_preview) {
        (Some((pane, terminal)), _) => {
            cli::app_do("pane.close", &json!({"pane": pane}))?;
            out["closed"] = json!(terminal);
        }
        (None, Some(pane)) => {
            cli::app_do("pane.close", &json!({"pane": pane}))?;
            out["closed_preview"] = json!(pane);
        }
        (None, None) => {}
    }
    Ok(out)
}

/// The direction to split nvim's pane (or its preview) in for a swap that
/// keeps its place: across the split that made the slot, so both nest in
/// the slot and the new pane keeps its size when the old one closes (along
/// it, the app shares the row out again). The slot
/// is always made by splitting the whole tab in `direction(position)`, so
/// across it is the other one. Not read from the layout: a header asks when
/// its pane is new, which is mid-swap, while it still shares a nested split
/// with the pane it replaces (and that answer would be the wrong way).
pub(crate) fn swap_direction() -> &'static str {
    match direction(config::load().sidebar.position) {
        "right" => "down",
        _ => "right",
    }
}

fn label(terminal: &str, tab: &str) -> Result<()> {
    rpc::call(
        "terminal.label",
        json!({"terminal": terminal, "labels": {LABEL: tab}}),
    )
    .map(drop)
}

/// Where a tab's nvim opens: the focused pane's own folder (`ctx`), exactly,
/// not its git repo. Each open `cd`s the tab's nvim there (`daemon::
/// start_sidebar`), so cmd-e always opens on the pwd of the pane it was
/// pressed in.
pub(crate) fn nvim_folder(ctx: &Ctx) -> PathBuf {
    ctx.cwd.clone()
}

/// The tab's sidebar, opened if it isn't.
pub(crate) fn ensure_open(ctx: &Ctx) -> Result<Opened> {
    match find(&ctx.tab)? {
        Some(terminal) => Ok(Opened {
            pane: terminal,
            notice: None,
        }),
        None => open(ctx),
    }
}

/// The TUI `Ctx` for an app invocation. Its folder is where the tab works
/// now, as the app's own Files panel sees it: the one the mod saw, else the
/// focused terminal's, else the tab's first terminal's (a mod pane has the
/// focus, such as the tree or a preview, or `open --tab` names the tab
/// alone). Only a tab without terminals gets its own or its space's folder
/// from the app's document: that is where the space was made, and a space
/// follows its terminals since (one made in one repo can work in another).
/// Never this process's own folder: the app's is often `/`.
pub(crate) fn ctx(app: &AppCtx) -> Result<Ctx> {
    let live = |terminal: &str| {
        host::terminal(terminal)
            .ok()
            .and_then(|facts| host::cwd_of(&facts))
    };
    let given = app
        .cwd
        .as_deref()
        .filter(|cwd| !cwd.is_empty())
        .map(PathBuf::from);
    let cwd = match given.or_else(|| app.terminal.as_deref().and_then(live)) {
        Some(cwd) => cwd,
        None => {
            let doc = doc::read()?;
            let first = doc
                .as_ref()
                .and_then(|doc| doc::terminals_of(doc, &app.tab).into_iter().next());
            first
                .as_deref()
                .and_then(live)
                .or_else(|| {
                    doc.as_ref()
                        .and_then(|doc| doc::folder_of(doc, &app.tab))
                        .map(PathBuf::from)
                })
                .with_context(|| {
                    format!(
                        "no folder for tab {}: no terminal in it, and neither the tab nor its space has a folder",
                        app.tab
                    )
                })?
        }
    };
    Ok(Ctx {
        workspace: app.space.clone(),
        tab: tab_id(&app.space, &app.tab),
        focused_pane: app.terminal.clone().unwrap_or_default(),
        cwd,
    })
}

/// What `toggle --state <open|close>` asks for; no `--state` flips it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Want {
    Flip,
    Open,
    Close,
}

/// Pure: the `--state` argument.
pub(crate) fn want_from_args(args: &[String]) -> Result<Want> {
    match cli::flag(args, "--state")? {
        None => Ok(Want::Flip),
        Some("open") => Ok(Want::Open),
        Some("close") => Ok(Want::Close),
        Some(other) => anyhow::bail!("--state is open or close, not {other:?}"),
    }
}

/// `herdr-nvim toggle [--state open|close] [--preview <file>] [--ctx JSON]`:
/// open or close this tab's nvim pane. With `--state` it is idempotent: the
/// Files panel's controller says what it wants, never "the other way round".
/// A preview in nvim's place counts as the open pane: closing closes it, and
/// `--state open` puts nvim back in its place (the preview's Source);
/// `--preview` (what the controller remembered Files closed) opens that
/// preview instead of nvim. Prints `{"terminal": "tN"}` (nvim opened),
/// `{"preview": pane}` (preview opened) or `{"closed": "tN"}`. Daemons whose
/// tab is gone are stopped by the watcher (`app-watch`), not here: a click
/// never waits on them.
pub fn toggle_cmd() -> Result<()> {
    let args: Vec<String> = env::args().skip(2).collect();
    let want = want_from_args(&args)?;
    let ctx = ctx(&cli::read_ctx(&args)?)?;
    let remembered = cli::flag(&args, "--preview")?
        .filter(|file| std::path::Path::new(file).is_file())
        .map(str::to_owned);
    let terminal = find(&ctx.tab)?;
    let preview = doc::read()?.and_then(|doc| doc::preview_of(&doc, raw_tab(&ctx.tab)));
    let shown = terminal.is_some() || preview.is_some();
    let made = match (shown, want) {
        (true, Want::Flip | Want::Close) => {
            let mut made = json!({});
            if let Some((pane, _)) = preview {
                cli::app_do("pane.close", &json!({"pane": pane}))?;
                made["closed_preview"] = json!(pane);
            }
            if let Some(terminal) = terminal {
                close(&terminal)?;
                made["closed"] = json!(terminal);
            }
            made
        }
        (false, Want::Flip | Want::Open) => match remembered {
            Some(file) => open_preview(&ctx, &file)?,
            None => {
                let opened = open(&ctx)?;
                match opened.notice {
                    Some(notice) => json!({"terminal": opened.pane, "notice": notice}),
                    None => json!({"terminal": opened.pane}),
                }
            }
        },
        // A preview alone: nvim back in its place (`open` swaps it), which is
        // the preview's Source.
        (true, Want::Open) => match terminal {
            Some(terminal) => json!({"terminal": terminal}),
            None => {
                let opened = open(&ctx)?;
                json!({"terminal": opened.pane})
            }
        },
        (false, Want::Close) => json!({}),
    };
    println!("{made}");
    Ok(())
}

/// `herdr-nvim app-preview --file <path> --ctx JSON` (Neovim's
/// `:HerdrPreview`): render the file in the tab's Neovim pane.
pub fn preview_cmd() -> Result<()> {
    let args: Vec<String> = env::args().skip(2).collect();
    let file = cli::flag(&args, "--file")?
        .context("usage: herdr-nvim app-preview --file <path> --ctx JSON")?;
    let ctx = ctx(&cli::read_ctx(&args)?)?;
    let file = crate::pathkey::absolute(file, &ctx.cwd);
    if !file.is_file() {
        anyhow::bail!("no such file: {}", file.display());
    }
    println!("{}", open_preview(&ctx, &file.to_string_lossy())?);
    Ok(())
}

/// Inside the sidebar terminal: which tab it belongs to (the app labels its
/// terminals), its own address, and the folder to start nvim in.
pub(crate) fn identity() -> Result<(TabId, String, PathBuf)> {
    let me = env::var("HC_TERMINAL_ID")
        .context("herdr-nvim sidebar must run in a Herdr app terminal (HC_TERMINAL_ID)")?;
    let facts = host::terminal(&me)?;
    let tab = host::tab_of(&facts).context("this terminal is in no Herdr app tab")?;
    label(&me, raw_tab(&tab))?;
    let args: Vec<String> = env::args().collect();
    let cwd = cli::flag(&args, "--cwd")?
        .map(PathBuf::from)
        .or_else(|| host::cwd_of(&facts))
        .context("no folder for the sidebar")?;
    Ok((TabId::new(tab), me, cwd))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    // A tab's nvim opens in the focused pane's own folder, even deep in a
    // repo: never its repo root.
    #[test]
    fn nvim_opens_in_the_focused_panes_own_folder() {
        let at = |cwd: &str| Ctx {
            workspace: "sp".into(),
            tab: tab_id("sp", "tb"),
            focused_pane: String::new(),
            cwd: PathBuf::from(cwd),
        };
        assert_eq!(
            nvim_folder(&at("/repo/src/deep")),
            PathBuf::from("/repo/src/deep")
        );
        assert_eq!(nvim_folder(&at("/plain")), PathBuf::from("/plain"));
    }

    #[test]
    fn state_argument_makes_the_toggle_idempotent() {
        assert_eq!(want_from_args(&args(&["--ctx", "{}"])).unwrap(), Want::Flip);
        assert_eq!(
            want_from_args(&args(&["--state", "open"])).unwrap(),
            Want::Open
        );
        assert_eq!(
            want_from_args(&args(&["--state", "close"])).unwrap(),
            Want::Close
        );
        assert!(want_from_args(&args(&["--state", "maybe"])).is_err());
        assert!(want_from_args(&args(&["--state"])).is_err());
    }
}
