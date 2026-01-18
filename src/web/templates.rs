//! HTML templates for the web interface.
//!
//! Embedded HTML templates for the configuration UI.

use crate::config::{Config, SchedulePlan, Weekday};

/// Generate HTML for schedule plans data (as JSON for JavaScript)
fn render_schedule_plans_json(plans: &[SchedulePlan]) -> String {
    serde_json::to_string(plans).unwrap_or_else(|_| "[]".to_string())
}

/// Generate HTML for day assignments data (as JSON for JavaScript)
fn render_day_assignments_json(config: &Config) -> String {
    let assignments: Vec<(&str, &str)> = Weekday::all()
        .iter()
        .map(|day| {
            let plan_name = config
                .day_assignments
                .get(day)
                .map(|s| s.as_str())
                .unwrap_or("Default");
            (day.short_name(), plan_name)
        })
        .collect();
    serde_json::to_string(&assignments).unwrap_or_else(|_| "[]".to_string())
}

/// Get the current active period info for display
fn get_active_period_info(config: &Config) -> String {
    let weekday = Config::get_current_weekday();
    let plan_name = config
        .get_current_plan()
        .map(|p| p.name.as_str())
        .unwrap_or("None");

    if let Some(period) = config.get_current_period() {
        format!(
            "{} → '{}': {} - {} (every {} min)",
            weekday.display_name(),
            plan_name,
            period.start_time,
            period.end_time,
            period.interval_min
        )
    } else {
        format!("{} → No active schedule", weekday.display_name())
    }
}

/// Render the main configuration page
pub fn render_config_page(config: &Config, status_message: Option<&str>) -> String {
    let status_html = status_message
        .map(|msg| format!(r#"<div class="alert">{}</div>"#, msg))
        .unwrap_or_default();

    let active_period = get_active_period_info(config);
    let current_interval = config.get_current_interval();
    let schedule_plans_json = render_schedule_plans_json(&config.schedule_plans);
    let day_assignments_json = render_day_assignments_json(config);

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Pi Zero W ePaper Display</title>
    <style>
        * {{ box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .container {{ max-width: 800px; margin: 0 auto; background: white; padding: 24px; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; margin-top: 0; }}
        h3 {{ color: #444; margin-top: 24px; margin-bottom: 12px; }}
        .status {{ background: #e3f2fd; padding: 16px; border-radius: 8px; margin-bottom: 20px; font-size: 14px; word-break: break-word; }}
        .alert {{ background: #c8e6c9; padding: 12px; border-radius: 8px; margin-bottom: 16px; color: #2e7d32; }}
        label {{ display: block; margin-top: 16px; font-weight: 600; color: #555; }}
        input, select {{ width: 100%; padding: 10px; margin-top: 6px; border: 1px solid #ddd; border-radius: 8px; font-size: 15px; }}
        input:focus, select:focus {{ outline: none; border-color: #2196F3; }}
        .checkbox-group {{ display: flex; gap: 20px; margin-top: 8px; flex-wrap: wrap; }}
        .checkbox-group label {{ display: flex; align-items: center; gap: 8px; font-weight: normal; margin-top: 0; }}
        .buttons {{ display: flex; gap: 10px; margin-top: 24px; flex-wrap: wrap; }}
        button {{ padding: 10px 20px; border: none; border-radius: 8px; font-size: 15px; cursor: pointer; font-weight: 600; }}
        .btn-primary {{ background: #4CAF50; color: white; }}
        .btn-blue {{ background: #2196F3; color: white; }}
        .btn-orange {{ background: #FF9800; color: white; }}
        .btn-red {{ background: #f44336; color: white; }}
        .btn-small {{ padding: 6px 12px; font-size: 13px; }}
        .btn-gray {{ background: #9e9e9e; color: white; }}
        button:hover {{ opacity: 0.9; }}
        hr {{ border: none; border-top: 1px solid #eee; margin: 24px 0; }}
        .actions {{ display: flex; gap: 10px; flex-wrap: wrap; }}
        .actions a {{ text-decoration: none; }}
        .help-text {{ color: #666; font-size: 13px; margin-top: 4px; }}
        textarea.url-input {{ width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 6px; box-sizing: border-box; font-family: inherit; font-size: 14px; resize: vertical; min-height: 80px; }}
        .row {{ display: flex; gap: 10px; }}
        .row input {{ flex: 1; }}
        /* Tabs */
        .tabs {{ display: flex; gap: 4px; border-bottom: 2px solid #e0e0e0; margin-top: 12px; flex-wrap: wrap; }}
        .tab {{ padding: 8px 16px; cursor: pointer; border-radius: 8px 8px 0 0; background: #f0f0f0; font-weight: 500; font-size: 14px; }}
        .tab.active {{ background: #2196F3; color: white; }}
        .tab-add {{ background: #e8f5e9; color: #2e7d32; }}
        .tab-content {{ display: none; padding: 16px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .tab-content.active {{ display: block; }}
        /* Schedule table */
        .schedule-table {{ width: 100%; border-collapse: collapse; margin-top: 8px; }}
        .schedule-table th {{ text-align: left; padding: 8px; background: #f5f5f5; font-size: 13px; }}
        .schedule-table td {{ padding: 6px; }}
        .schedule-table input[type="time"] {{ width: 100px; padding: 6px; }}
        .schedule-table input[type="number"] {{ width: 70px; padding: 6px; }}
        .schedule-controls {{ display: flex; gap: 8px; margin-top: 8px; flex-wrap: wrap; }}
        .preset-btn {{ padding: 6px 12px; font-size: 12px; background: #e0e0e0; color: #333; }}
        /* Day assignments */
        .day-grid {{ display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; margin-top: 12px; }}
        .day-card {{ text-align: center; padding: 10px 4px; border: 2px solid #e0e0e0; border-radius: 8px; background: #fafafa; }}
        .day-card.today {{ border-color: #4CAF50; background: #e8f5e9; }}
        .day-card .day-name {{ font-weight: 600; font-size: 13px; color: #333; margin-bottom: 6px; }}
        .day-card select {{ width: 100%; padding: 4px; font-size: 12px; border-radius: 4px; }}
        .plan-name-input {{ margin-bottom: 12px; }}
        details {{ margin-top: 16px; }}
        details summary {{ cursor: pointer; font-weight: 600; color: #555; padding: 8px 0; }}
        /* Footer */
        .footer {{ margin-top: 24px; padding-top: 16px; border-top: 1px solid #eee; text-align: center; font-size: 13px; color: #888; }}
        .footer a {{ color: #666; text-decoration: none; }}
        .footer a:hover {{ color: #333; text-decoration: underline; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🖼️ Pi Zero W ePaper Display</h1>
        {status_html}
        <div class="status">
            <strong>URL:</strong> <a href="{url}" target="_blank" style="color: #1565c0;">{url_display}</a><br>
            <strong>Active:</strong> {active_period} &nbsp;|&nbsp; <strong>Interval:</strong> {current_interval} min<br>
            <strong>Size:</strong> {display_width}×{display_height} &nbsp;|&nbsp; <strong>Rotation:</strong> {rotation}°
        </div>
        <form method="POST" action="/save" id="configForm">
            <label>Image URL:</label>
            <textarea name="image_url" class="url-input" rows="3" placeholder="https://example.com/image.png">{url}</textarea>
            <div class="help-text">Enter the full URL to the image. Long URLs (e.g., Grafana render URLs) are supported.</div>

            <h3>📅 Schedule Plans</h3>
            <div class="help-text">Create named schedule plans and assign them to different days of the week.</div>

            <div class="day-grid" id="dayAssignments"></div>

            <div class="tabs" id="planTabs"></div>
            <div id="planContents"></div>

            <h3>⚙️ Display Settings</h3>
            <label>Dimensions:</label>
            <div class="row">
                <input type="number" name="display_width" value="{display_width}" min="100" max="2000" placeholder="Width">
                <input type="number" name="display_height" value="{display_height}" min="100" max="2000" placeholder="Height">
            </div>

            <label>Rotation:</label>
            <select name="rotation">
                <option value="0" {sel0}>0° (No rotation)</option>
                <option value="90" {sel90}>90° Clockwise</option>
                <option value="180" {sel180}>180° (Upside down)</option>
                <option value="270" {sel270}>270° Clockwise</option>
            </select>

            <label>Transform Order:</label>
            <select name="rotate_first">
                <option value="1" {rot_first_yes}>Rotate then Mirror</option>
                <option value="0" {rot_first_no}>Mirror then Rotate</option>
            </select>

            <label>Options:</label>
            <div class="checkbox-group">
                <label><input type="checkbox" name="mirror_h" {mirror_h}> Mirror H</label>
                <label><input type="checkbox" name="mirror_v" {mirror_v}> Mirror V</label>
                <label><input type="checkbox" name="scale_to_fit" {scale_to_fit}> Scale to Fit</label>
            </div>

            <div class="buttons">
                <button type="submit" class="btn-primary">Save</button>
                <button type="submit" formaction="/apply" class="btn-blue">Save &amp; Apply</button>
            </div>
        </form>
        <hr>
        <h3>Actions</h3>
        <div class="actions">
            <a href="/action/show"><button type="button" class="btn-orange">Refresh Now</button></a>
            <a href="/action/test"><button type="button" class="btn-blue">Test Pattern</button></a>
            <a href="/action/clear"><button type="button" class="btn-red">Clear Display</button></a>
        </div>

        <details>
            <summary>ℹ️ Help</summary>
            <div style="background:#fafafa;padding:16px;border-radius:8px;margin-top:8px;font-size:13px;">
                <p><strong>Schedule Plans:</strong> Create named schedules (e.g., "Weekday", "Weekend") with different time periods. Assign plans to days of the week.</p>
                <p><strong>Time Periods:</strong> Each plan must cover all 24 hours. Use 00:00-00:00 for a single all-day period.</p>
                <p><strong>Display:</strong> Waveshare 7.3" E-Paper, 800×480, 6-color (Black, White, Red, Yellow, Blue, Green).</p>
            </div>
        </details>
    </div>
    <script>
    const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
    const DAY_NAMES = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
    let plans = {schedule_plans_json};
    let dayAssignments = Object.fromEntries({day_assignments_json});
    let activePlanIdx = 0;

    function renderAll() {{
        renderDayAssignments();
        renderTabs();
        renderPlanContent();
    }}

    function renderDayAssignments() {{
        const container = document.getElementById('dayAssignments');
        const today = new Date().getDay();
        const todayIdx = today === 0 ? 6 : today - 1;
        container.innerHTML = DAYS.map((day, i) => `
            <div class="day-card ${{i === todayIdx ? 'today' : ''}}">
                <div class="day-name">${{day}}</div>
                <select name="day_${{day}}" onchange="dayAssignments['${{day}}']=this.value">
                    ${{plans.map(p => `<option value="${{p.name}}" ${{dayAssignments[day]===p.name?'selected':''}}>${{p.name}}</option>`).join('')}}
                </select>
            </div>
        `).join('');
    }}

    function renderTabs() {{
        const container = document.getElementById('planTabs');
        container.innerHTML = plans.map((p, i) =>
            `<div class="tab ${{i===activePlanIdx?'active':''}}" onclick="selectPlan(${{i}})">${{p.name}}</div>`
        ).join('') + `<div class="tab tab-add" onclick="addPlan()">+ New Plan</div>`;
    }}

    function renderPlanContent() {{
        const container = document.getElementById('planContents');
        container.innerHTML = plans.map((plan, pi) => `
            <div class="tab-content ${{pi===activePlanIdx?'active':''}}" id="plan_${{pi}}">
                <input type="hidden" name="plan_name_${{pi}}" value="${{plan.name}}">
                <div class="plan-name-input">
                    <label style="display:inline;margin:0;">Plan Name:</label>
                    <input type="text" value="${{plan.name}}" style="width:200px;display:inline;margin-left:8px;"
                           onchange="renamePlan(${{pi}}, this.value)" ${{plans.length===1?'readonly':''}}>
                    ${{plans.length > 1 ? `<button type="button" class="btn-small btn-red" style="margin-left:8px;" onclick="deletePlan(${{pi}})">Delete Plan</button>` : ''}}
                </div>
                <table class="schedule-table">
                    <thead><tr><th>Start</th><th>End</th><th>Interval (min)</th><th></th></tr></thead>
                    <tbody id="periods_${{pi}}">
                        ${{plan.periods.map((p, ri) => renderPeriodRow(pi, ri, p)).join('')}}
                    </tbody>
                </table>
                <div class="schedule-controls">
                    <button type="button" class="btn-small btn-blue" onclick="addPeriod(${{pi}})">+ Add Period</button>
                    <button type="button" class="btn-small preset-btn" onclick="setPreset(${{pi}},'simple')">Simple</button>
                    <button type="button" class="btn-small preset-btn" onclick="setPreset(${{pi}},'daynight')">Day/Night</button>
                    <button type="button" class="btn-small preset-btn" onclick="setPreset(${{pi}},'work')">Work Hours</button>
                </div>
            </div>
        `).join('');
        syncHiddenFields();
    }}

    function renderPeriodRow(pi, ri, period) {{
        return `<tr>
            <td><input type="time" value="${{period.start_time}}" onchange="updatePeriod(${{pi}},${{ri}},'start_time',this.value)"></td>
            <td><input type="time" value="${{period.end_time}}" onchange="updatePeriod(${{pi}},${{ri}},'end_time',this.value)"></td>
            <td><input type="number" value="${{period.interval_min}}" min="1" max="1440" onchange="updatePeriod(${{pi}},${{ri}},'interval_min',parseInt(this.value))"></td>
            <td><button type="button" class="btn-small btn-red" onclick="removePeriod(${{pi}},${{ri}})">✕</button></td>
        </tr>`;
    }}

    function selectPlan(idx) {{ activePlanIdx = idx; renderTabs(); renderPlanContent(); }}

    function addPlan() {{
        const name = prompt('Enter plan name:', 'New Plan');
        if (name && !plans.find(p => p.name === name)) {{
            plans.push({{ name: name, periods: [{{ start_time: '00:00', end_time: '00:00', interval_min: 60 }}] }});
            activePlanIdx = plans.length - 1;
            renderAll();
        }} else if (name) {{ alert('Plan name already exists.'); }}
    }}

    function renamePlan(idx, newName) {{
        if (!newName.trim()) return;
        const oldName = plans[idx].name;
        if (plans.find((p,i) => i !== idx && p.name === newName)) {{ alert('Name exists.'); return; }}
        plans[idx].name = newName;
        Object.keys(dayAssignments).forEach(d => {{ if (dayAssignments[d] === oldName) dayAssignments[d] = newName; }});
        renderAll();
    }}

    function deletePlan(idx) {{
        if (plans.length <= 1) return;
        const name = plans[idx].name;
        const fallback = plans.find((p,i) => i !== idx).name;
        Object.keys(dayAssignments).forEach(d => {{ if (dayAssignments[d] === name) dayAssignments[d] = fallback; }});
        plans.splice(idx, 1);
        activePlanIdx = Math.min(activePlanIdx, plans.length - 1);
        renderAll();
    }}

    function addPeriod(pi) {{
        plans[pi].periods.push({{ start_time: '00:00', end_time: '00:00', interval_min: 60 }});
        renderPlanContent();
    }}

    function removePeriod(pi, ri) {{
        if (plans[pi].periods.length > 1) {{ plans[pi].periods.splice(ri, 1); renderPlanContent(); }}
        else {{ alert('At least one period required.'); }}
    }}

    function updatePeriod(pi, ri, field, value) {{
        plans[pi].periods[ri][field] = value;
        syncHiddenFields();
    }}

    function setPreset(pi, preset) {{
        if (preset === 'simple') plans[pi].periods = [{{ start_time: '00:00', end_time: '00:00', interval_min: 60 }}];
        else if (preset === 'daynight') plans[pi].periods = [{{ start_time: '06:00', end_time: '22:00', interval_min: 30 }}, {{ start_time: '22:00', end_time: '06:00', interval_min: 120 }}];
        else if (preset === 'work') plans[pi].periods = [{{ start_time: '00:00', end_time: '07:00', interval_min: 120 }}, {{ start_time: '07:00', end_time: '19:00', interval_min: 15 }}, {{ start_time: '19:00', end_time: '00:00', interval_min: 60 }}];
        renderPlanContent();
    }}

    function syncHiddenFields() {{
        let existing = document.getElementById('plansData');
        if (existing) existing.remove();
        const input = document.createElement('input');
        input.type = 'hidden'; input.name = 'plans_json'; input.id = 'plansData';
        input.value = JSON.stringify({{ plans: plans, day_assignments: dayAssignments }});
        document.getElementById('configForm').appendChild(input);
    }}

    renderAll();
    </script>
    <div class="footer">
        <a href="https://github.com/bolausson/RPiZeroW-ePaper-Display" target="_blank">🔗 GitHub Repository</a>
    </div>
</body>
</html>"##,
        status_html = status_html,
        url = html_escape(&config.image_url),
        url_display = truncate_url(&config.image_url, 60),
        schedule_plans_json = schedule_plans_json,
        day_assignments_json = day_assignments_json,
        active_period = active_period,
        current_interval = current_interval,
        display_width = config.display_width,
        display_height = config.display_height,
        rotation = config.rotation,
        sel0 = selected_if(config.rotation == 0),
        sel90 = selected_if(config.rotation == 90),
        sel180 = selected_if(config.rotation == 180),
        sel270 = selected_if(config.rotation == 270),
        rot_first_yes = selected_if(config.rotate_first),
        rot_first_no = selected_if(!config.rotate_first),
        mirror_h = checked_if(config.mirror_h),
        mirror_v = checked_if(config.mirror_v),
        scale_to_fit = checked_if(config.scale_to_fit),
    )
}

/// Render a simple message page
pub fn render_message_page(title: &str, message: &str, back_link: bool) -> String {
    let back_html = if back_link {
        r#"<p><a href="/">← Back to configuration</a></p>"#
    } else {
        ""
    };

    format!(
        r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>{title}</title>
<style>body{{font-family:sans-serif;padding:20px;}}
.msg{{background:#e3f2fd;padding:20px;border-radius:8px;max-width:500px;}}
a{{color:#2196F3;}}</style></head>
<body><div class="msg"><h2>{title}</h2><p>{message}</p>{back_html}</div></body></html>"#,
        title = title,
        message = message,
        back_html = back_html,
    )
}

fn selected_if(condition: bool) -> &'static str {
    if condition { "selected" } else { "" }
}

fn checked_if(condition: bool) -> &'static str {
    if condition { "checked" } else { "" }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn truncate_url(url: &str, max_len: usize) -> String {
    let escaped = html_escape(url);
    if escaped.len() <= max_len {
        escaped
    } else {
        format!("{}…", &escaped[..max_len])
    }
}
