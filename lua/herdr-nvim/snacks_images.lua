local M = {}

function M.setup()
  vim.api.nvim_create_autocmd("UIEnter", {
    group = vim.api.nvim_create_augroup("herdr-nvim-snacks-images", { clear = true }),
    callback = vim.schedule_wrap(function()
      local images = package.loaded["snacks.image.image"]
      local placement = package.loaded["snacks.image.placement"]
      if type(images) ~= "table" or type(images.clear) ~= "function"
        or type(placement) ~= "table" or type(placement.clean) ~= "function" then
        return
      end

      local buffers = {}
      for _, buf in ipairs(vim.api.nvim_list_bufs()) do
        local group = "snacks.image.inline." .. buf
        if vim.fn.exists("#" .. group) == 1 then
          buffers[#buffers + 1] = { buf = buf, group = group }
        end
      end
      if #buffers == 0 then return end

      -- A headless daemon can mark Kitty images as sent before a TUI exists.
      -- Rebuild placements for the attached TUI using the PNGs already on disk.
      images.clear()
      for _, entry in ipairs(buffers) do
        placement.clean(entry.buf)
        if #vim.fn.win_findbuf(entry.buf) > 0 then
          vim.api.nvim_exec_autocmds("BufWinEnter", { group = entry.group, buffer = entry.buf })
        end
      end
    end),
  })
end

return M
