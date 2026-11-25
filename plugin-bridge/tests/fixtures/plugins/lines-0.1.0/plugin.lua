-- `lines`: a pact plugin the bridge's own tests load, written in Lua so that pact-plugin-driver runs it
-- in the bridge's process, and no plugin binary has to be downloaded to test the bridge.
--
-- Its content matcher compares a `text/x-lines` body line by line. A line is equal to the expected one,
-- or matches the `regex` rule at its path (`$[0]`, `$[1]`, ...), read as a Lua pattern. An interaction configured with
-- `ignore-case` compares lines ignoring case. Its transport entry is declared and never called: it is
-- there to show what the bridge describes and does not contribute.

function init(implementation, version)
  local types = { ["content-types"] = "text/x-lines;application/x-lines\\+text" }
  return {
    { entryType = "CONTENT_MATCHER", key = "lines", values = types },
    { entryType = "TRANSPORT", key = "lines", values = {} },
  }
end

local function split(text)
  local lines = {}
  for line in (text or ""):gmatch("([^\n]*)\n?") do
    table.insert(lines, line)
  end
  if lines[#lines] == "" then
    table.remove(lines)
  end
  return lines
end

function match_contents(request)
  local expected = split(request.expected and request.expected.contents)
  local actual = split(request.actual and request.actual.contents)
  local config = request.plugin_configuration or {}
  local interaction = config.interaction_configuration or {}
  local ignore_case = interaction["ignore-case"] == true
  local mismatches = {}

  if #actual ~= #expected then
    mismatches["$"] = {
      mismatch = "Expected " .. #expected .. " line(s) but got " .. #actual,
      expected = tostring(#expected),
      actual = tostring(#actual),
    }
  end
  for index, line in ipairs(expected) do
    local path = "$[" .. (index - 1) .. "]"
    local got = actual[index]
    if got ~= nil then
      local rules = (request.rules or {})[path] or {}
      local regex = nil
      for _, rule in ipairs(rules) do
        if rule.type == "regex" then
          regex = rule.values and rule.values.regex
        end
      end
      if regex ~= nil then
        if got:match("^" .. regex .. "$") == nil then
          mismatches[path] = { mismatch = "Expected '" .. got .. "' to match '" .. regex .. "'", expected = line, actual = got }
        end
      else
        local a, b = line, got
        if ignore_case then
          a, b = a:lower(), b:lower()
        end
        if a ~= b then
          mismatches[path] = { mismatch = "Expected '" .. line .. "' but got '" .. got .. "'", expected = line, actual = got }
        end
      end
    end
  end
  return { mismatches = mismatches }
end

function configure_interaction(content_type, config)
  error("the lines plugin configures no interaction")
end
