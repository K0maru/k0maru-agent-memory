--!nonstrict

-- K0maru Agent Memory -- Tern host half.
--
-- Implements the `cockpit` block (a Living ADR + doctor dashboard) and the
-- `cli` lens (structured rendering for `k0maru ...` and
-- `k0maru-memory ...` output). The window half -- palette commands and the
-- plugin `recall` export -- lives in window.luau.

type CockpitState = {
	cwd: string?,
	loading: boolean,
	doctor: any,
	guard: any,
	error: string?,
}

type CliState = {
	run: LensRun,
	lines: { string },
	status: number?,
}

local PROBE_TIMEOUT_MS = 30000
local MAX_DETAIL = 800

-- Which cockpit views still wait for a probe (keyed by state, so several
-- cockpits in one host VM do not share a counter).
local pending = setmetatable({}, { __mode = "k" }) :: { [CockpitState]: number }

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

local function first_line(text: string?): string
	if text == nil then
		return ""
	end
	local line = string.match(text, "^[^\r\n]*") or ""
	if #line > MAX_DETAIL then
		line = string.sub(line, 1, MAX_DETAIL) .. "..."
	end
	return line
end

local function describe_failure(label: string, res: ProcessResult): string
	local detail = res.stderr
	if detail == nil or detail == "" then
		detail = res.stdout
	end
	detail = first_line(detail)
	if detail ~= "" then
		return string.format("%s failed (exit %d): %s", label, res.status, detail)
	end
	return string.format("%s failed (exit %d)", label, res.status)
end

local function summary_text(report: any): string
	local summary = report.summary or {}
	return string.format(
		"%d passed - %d warnings - %d failures",
		summary.passed or 0,
		summary.warnings or 0,
		summary.failures or 0
	)
end

local function doctor_tone(report: any): string
	local summary = report.summary or {}
	if (summary.failures or 0) > 0 then
		return "bad"
	end
	if (summary.warnings or 0) > 0 then
		return "warn"
	end
	return "ok"
end

local function doctor_badge(report: any): Node
	if report == nil then
		return tern.ui.badge("Doctor pending", "muted")
	end
	return tern.ui.badge(summary_text(report), doctor_tone(report))
end

local function doctor_ratio(report: any): number
	local summary = report.summary or {}
	local total = summary.total or 0
	if total <= 0 then
		return 1
	end
	return (summary.passed or 0) / total
end

local function guard_badge(report: any): Node
	if report == nil then
		return tern.ui.badge("Guard pending", "muted")
	end
	local violations = report.violations or {}
	local count = #violations
	if count > 0 then
		return tern.ui.badge(
			string.format("Guard %d violation%s", count, count == 1 and "" or "s"),
			"bad"
		)
	end
	if report.is_passed then
		return tern.ui.badge("Guard PASS", "ok")
	end
	return tern.ui.badge("Guard FAIL", "bad")
end

local function guard_has_violations(report: any): boolean
	return report ~= nil and #(report.violations or {}) > 0
end

local STATUS_LABELS = {
	pass = "PASS",
	warn = "WARN",
	fail = "FAIL",
}

local STATUS_TONES = {
	pass = "ok",
	warn = "warn",
	fail = "bad",
}

local function detail_spans(item: any): Spans
	local spans: { any } = { tostring(item.detail or "") }
	local suggestion = item.suggestion
	if suggestion ~= nil and suggestion ~= "" then
		table.insert(spans, " -- ")
		table.insert(spans, tern.ui.span(tostring(suggestion), "muted"))
	end
	return spans
end

local function diagnostic_rows(items: { any }): { { Cell } }
	local rows: { { Cell } } = {}
	for _, item in ipairs(items) do
		local status = tostring(item.status or "")
		table.insert(rows, {
			tostring(item.category or ""),
			tostring(item.name or ""),
			{ tern.ui.span(STATUS_LABELS[status] or string.upper(status), STATUS_TONES[status] or "muted") },
			detail_spans(item),
		})
	end
	return rows
end

local DIAGNOSTIC_COLUMNS: { Col } = {
	{ id = "category", head = "Category", priority = 0 },
	{ id = "check", head = "Check", grow = 1, priority = 1 },
	{ id = "status", head = "Status", priority = 2 },
	{ id = "detail", head = "Detail", grow = 2, priority = 0 },
}

local function violation_diag(violation: any): Diag
	local rule_id = violation.rule_id ~= nil and tostring(violation.rule_id) or nil
	local message = tostring(violation.rule_title or rule_id or "Invariant violation")
	if rule_id ~= nil and rule_id ~= "" then
		message = string.format("[%s] %s", rule_id, message)
	end
	if violation.matched_content ~= nil and violation.matched_content ~= "" then
		message = message .. ": " .. tostring(violation.matched_content)
	end

	local notes: { string } = {}
	if violation.source_note ~= nil and violation.source_note ~= "" then
		table.insert(notes, "Source: " .. tostring(violation.source_note))
	end
	if violation.remediation ~= nil and violation.remediation ~= "" then
		table.insert(notes, "Fix: " .. tostring(violation.remediation))
	end

	local line = violation.line_number
	return {
		severity = violation.severity == "warn" and "warning" or "error",
		code = rule_id,
		message = message,
		path = violation.file_path ~= nil and tostring(violation.file_path) or nil,
		line = type(line) == "number" and line or nil,
		notes = #notes > 0 and notes or nil,
	}
end

local function violation_children(violations: { any }, cwd: string?): { Node }
	local children: { Node } = {}
	for _, violation in ipairs(violations) do
		table.insert(children, tern.ui.diagnostic(violation_diag(violation), cwd))
	end
	return children
end

local function header_card(state: CockpitState): Node
	local header: { Node } = {
		tern.ui.row({
			guard_badge(state.guard),
			doctor_badge(state.doctor),
		}),
	}

	if state.doctor ~= nil and state.doctor.summary ~= nil then
		table.insert(
			header,
			tern.ui.table(
				{
					{ id = "summary", head = "Doctor", grow = 1 },
					{ id = "health", head = "Health", align = "end" },
				},
				{
					{
						tostring(state.doctor.summary.total or 0) .. " checks",
						tern.ui.meter_cell(doctor_ratio(state.doctor)),
					},
				}
			)
		)
	end

	return tern.ui.card({
		tern.ui.span("K0maru Memory Cockpit", "strong"),
		tern.ui.span(state.cwd ~= nil and ("  " .. state.cwd) or "", "muted"),
	}, header)
end

local function refresh(state: CockpitState, cx: BlockCx)
	state.loading = true
	state.error = nil
	pending[state] = 2

	local function settle()
		pending[state] = (pending[state] or 1) - 1
		state.loading = pending[state] > 0
		cx:render()
	end

	tern.process.run(
		{ "k0maru", "doctor", "--json" },
		{ cwd = state.cwd, timeout_ms = PROBE_TIMEOUT_MS },
		function(res: ProcessResult)
			local report = decode_json(res.stdout)
			if report ~= nil and report.items ~= nil and report.summary ~= nil then
				state.doctor = report
			else
				state.error = describe_failure("k0maru doctor", res)
			end
			settle()
		end
	)

	tern.process.run(
		{ "k0maru", "guard", "check", "--json" },
		{ cwd = state.cwd, timeout_ms = PROBE_TIMEOUT_MS },
		function(res: ProcessResult)
			-- `guard check` exits 1 when a deny rule fires, but still prints the report.
			local report = decode_json(res.stdout)
			if report ~= nil and report.checked_files ~= nil and report.is_passed ~= nil then
				state.guard = report
			else
				state.error = describe_failure("k0maru guard check", res)
			end
			settle()
		end
	)
end

tern.block.define("cockpit", {
	init = function(cx: BlockCx, args: { string }, saved: any): CockpitState
		local state: CockpitState = {
			cwd = cx.cwd,
			loading = true,
			doctor = nil,
			guard = nil,
			error = nil,
		}
		refresh(state, cx)
		return state
	end,

	title = function(state: CockpitState): string
		return "K0maru Memory Cockpit"
	end,

	key = function(state: CockpitState, key: Key, cx: BlockCx): boolean
		if key.name == "r" then
			refresh(state, cx)
			return true
		end
		return false
	end,

	view = function(state: CockpitState, cx: BlockCx): BlockView
		local children: { Node } = { header_card(state) }

		if state.error ~= nil then
			table.insert(
				children,
				tern.ui.card({ tern.ui.span("Error", "strong bad") }, {
					tern.ui.text({ state.error }),
					tern.ui.text({ tern.ui.span("Press r to retry.", "muted") }),
				})
			)
		end

		if state.loading and state.doctor == nil and state.guard == nil then
			table.insert(
				children,
				tern.ui.text({ tern.ui.span("Running k0maru doctor and guard check...", "muted") })
			)
		end

		if state.doctor ~= nil and state.doctor.items ~= nil and #state.doctor.items > 0 then
			table.insert(
				children,
				tern.ui.table(DIAGNOSTIC_COLUMNS, diagnostic_rows(state.doctor.items))
			)
		end

		if guard_has_violations(state.guard) then
			table.insert(
				children,
				tern.ui.card(
					{ tern.ui.span("Guard violations", "strong bad") },
					violation_children(state.guard.violations, state.cwd)
				)
			)
		end

		return { main = tern.ui.col(children) }
	end,
})

-- Lens ----------------------------------------------------------------------

local function guard_view(report: any): Node
	local violations = report.violations or {}
	local children: { Node } = {
		tern.ui.row({
			tern.ui.badge(report.is_passed and "PASS" or "FAIL", report.is_passed and "ok" or "bad"),
			tern.ui.text({
				tern.ui.span(
					string.format(
						"  %d files - %d hunks - %dms",
						report.checked_files or 0,
						report.checked_diff_hunks or 0,
						report.duration_ms or 0
					),
					"muted"
				),
			}),
		}),
	}

	if #violations > 0 then
		table.insert(children, tern.ui.rule({ tern.ui.span("Violations", "strong") }))
		for _, violation in ipairs(violations) do
			table.insert(children, tern.ui.diagnostic(violation_diag(violation)))
		end
	end

	return tern.ui.col({
		tern.ui.card({ tern.ui.span("Living ADR Guard", "strong") }, children),
	})
end

local function doctor_view(report: any): Node
	local summary = report.summary or {}
	local items = report.items or {}
	local children: { Node } = {
		tern.ui.test_summary(
			summary.passed or 0,
			summary.failures or 0,
			summary.warnings or 0
		),
	}

	if #items > 0 then
		table.insert(children, tern.ui.table(DIAGNOSTIC_COLUMNS, diagnostic_rows(items)))
	end

	return tern.ui.col({
		tern.ui.card({ tern.ui.span("K0maru Doctor", "strong") }, children),
	})
end

tern.lens.define("cli", {
	open = function(run: LensRun, cx: EffectCx): CliState
		return { run = run, lines = {}, status = nil }
	end,

	line = function(state: CliState, l: LensLine)
		table.insert(state.lines, l.text)
	end,

	finish = function(state: CliState, status: number)
		state.status = status
	end,

	view = function(state: CliState): Node?
		if state.status == nil then
			return nil
		end

		local raw = table.concat(state.lines, "\n")
		local trimmed = string.gsub(raw, "^%s+", "")
		local first = string.sub(trimmed, 1, 1)

		if first == "{" or first == "[" then
			local decoded = decode_json(raw)
			if decoded ~= nil then
				if decoded.checked_files ~= nil and decoded.is_passed ~= nil then
					return guard_view(decoded)
				end
				if decoded.summary ~= nil and decoded.items ~= nil then
					return doctor_view(decoded)
				end
			end
		end

		return tern.parse.json_tree(raw)
	end,
})
