--!nonstrict

-- K0maru Agent Memory -- Tern window half.
--
-- Registers the palette commands (open the cockpit, run the Living ADR
-- Guard) and the Carly `recall` export backed by `k0maru search --json`.
-- The block and lens implementations live in host.luau.

local SEARCH_TIMEOUT_MS = 20000
local GUARD_TIMEOUT_MS = 30000
local MAX_MESSAGE = 500

local function first_line(text: string?): string
	if text == nil then
		return ""
	end
	local line = string.match(text, "^[^\r\n]*") or ""
	if #line > MAX_MESSAGE then
		line = string.sub(line, 1, MAX_MESSAGE) .. "..."
	end
	return line
end

local function focused_cwd(cx: WindowCx): string?
	local pane_id = cx.session.focused()
	if pane_id == nil then
		return nil
	end
	local info = cx.session.pane(pane_id)
	if info == nil or info.cwd == nil or info.cwd == "" then
		return nil
	end
	return info.cwd
end

local function process_opts(cwd: string?, timeout_ms: number): ProcessOptions
	local opts: ProcessOptions = { timeout_ms = timeout_ms }
	if cwd ~= nil then
		opts.cwd = cwd
	end
	return opts
end

local function decode_json(text: string?): any
	if text == nil or text == "" then
		return nil
	end
	local ok, decoded = pcall(tern.json.decode, text)
	if ok and type(decoded) == "table" then
		return decoded
	end
	return nil
end

tern.command({
	id = "cockpit",
	title = "K0maru: Open Memory Cockpit",
	icon = "brain",
	group = "K0maru",
	run = function(cx: WindowCx)
		local pane = cx:new_block("k0maru-memory.cockpit", {}, "beside")
		if pane == nil then
			cx:toast("error", "K0maru cockpit unavailable", "The k0maru-memory plugin is not ready on this host.")
		end
	end,
})

tern.command({
	id = "guard_check",
	title = "K0maru: Run Living ADR Guard Check",
	icon = "shield",
	group = "K0maru",
	run = function(cx: WindowCx)
		cx:toast("info", "Running Living ADR Guard check...")
		tern.process.run(
			{ "k0maru", "guard", "check", "--json" },
			process_opts(focused_cwd(cx), GUARD_TIMEOUT_MS),
			function(res: ProcessResult, fresh: WindowCx?)
				local wcx = fresh or cx
				if wcx == nil then
					return
				end

				-- `guard check` exits 1 on deny violations but still prints a report.
				local report = decode_json(res.stdout)
				if report == nil or report.checked_files == nil or report.is_passed == nil then
					local detail = first_line(res.stderr)
					if detail == "" then
						detail = first_line(res.stdout)
					end
					if detail == "" then
						detail = string.format("exit %d", res.status)
					end
					wcx:toast("error", "K0maru Guard check failed", detail)
					return
				end

				local violations = report.violations or {}
				local checked = string.format(
					"%d files - %d hunks - %dms",
					report.checked_files or 0,
					report.checked_diff_hunks or 0,
					report.duration_ms or 0
				)
				if not report.is_passed then
					local first = violations[1]
					local sub = checked
					if first ~= nil then
						sub = string.format("%s: %s (%s)", checked, tostring(first.rule_id or "rule"), tostring(first.file_path or "?"))
					end
					wcx:toast(
						"error",
						string.format("K0maru Guard FAIL - %d violation%s", #violations, #violations == 1 and "" or "s"),
						sub
					)
				elseif #violations > 0 then
					wcx:toast(
						"success",
						string.format("K0maru Guard passed with %d warning%s", #violations, #violations == 1 and "" or "s"),
						checked
					)
				else
					wcx:toast("success", "K0maru Guard PASS", checked)
				end
			end
		)
	end,
})

tern.carly.export("recall", {
	sig = "(query: string) -> any",
	doc = "Search local LLM-Wiki / SecondBrain memory hub via k0maru search --json",
}, function(query: string, cx: WindowCx, call: ExportCall): ExportWait
	local cwd = focused_cwd(cx)

	tern.process.run(
		{ "k0maru", "search", tostring(query), "--json" },
		process_opts(cwd, SEARCH_TIMEOUT_MS),
		function(res: ProcessResult, fresh: WindowCx?)
			if call.cancelled then
				return
			end

			local results = decode_json(res.stdout)
			if results ~= nil then
				call:reply(results)
				return
			end

			if res.status ~= 0 or res.timed_out then
				local detail = first_line(res.stderr)
				if detail == "" then
					detail = first_line(res.stdout)
				end
				if detail == "" then
					detail = res.timed_out and "timed out" or string.format("exit %d", res.status)
				end
				call:fail(string.format("k0maru search failed: %s", detail))
				return
			end

			-- A query with no hits prints an empty array; keep a JSON-safe empty list.
			call:reply(tern.json.array())
		end
	)

	return call:wait(15000)
end)
