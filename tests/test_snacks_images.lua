local compat = require("herdr-nvim.snacks_images")

T.test("app UI attachment resends Snacks inline images only for visible buffers", function()
  local image_module = "snacks.image.image"
  local placement_module = "snacks.image.placement"
  local original_images = package.loaded[image_module]
  local original_placement = package.loaded[placement_module]
  local visible = vim.api.nvim_get_current_buf()
  local hidden = vim.api.nvim_create_buf(false, true)
  local refreshed = {}
  local cleaned = {}
  local groups = {}
  local cleared = 0

  local ok, err = pcall(function()
    for _, buf in ipairs({ visible, hidden }) do
      local group = vim.api.nvim_create_augroup("snacks.image.inline." .. buf, { clear = true })
      groups[#groups + 1] = group
      vim.api.nvim_create_autocmd("BufWinEnter", {
        group = group,
        buffer = buf,
        callback = function() refreshed[#refreshed + 1] = buf end,
      })
    end
    package.loaded[image_module] = { clear = function() cleared = cleared + 1 end }
    package.loaded[placement_module] = { clean = function(buf) cleaned[buf] = true end }

    compat.setup()
    compat.setup()
    vim.api.nvim_exec_autocmds("UIEnter", { group = "herdr-nvim-snacks-images" })
    T.ok(vim.wait(500, function() return #refreshed > 0 end))
    T.eq(cleared, 1)
    T.eq(cleaned, { [visible] = true, [hidden] = true })
    T.eq(refreshed, { visible })
  end)

  package.loaded[image_module] = original_images
  package.loaded[placement_module] = original_placement
  vim.api.nvim_del_augroup_by_name("herdr-nvim-snacks-images")
  for _, group in ipairs(groups) do vim.api.nvim_del_augroup_by_id(group) end
  vim.api.nvim_buf_delete(hidden, { force = true })
  if not ok then error(err) end
end)
