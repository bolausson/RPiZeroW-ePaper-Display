//! HTML templates for the web interface.
//!
//! Embedded HTML templates for the configuration UI.

use crate::config::Config;

/// Render the main configuration page
pub fn render_config_page(config: &Config, status_message: Option<&str>) -> String {
    let status_html = status_message
        .map(|msg| format!(r#"<div class="alert">{}</div>"#, msg))
        .unwrap_or_default();

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Pi Zero W ePaper Display</title>
    <style>
        * {{ box-sizing: border-box; }}
        body {{ 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0; padding: 20px; background: #f5f5f5;
        }}
        .container {{ 
            max-width: 600px; margin: 0 auto; background: white;
            padding: 24px; border-radius: 12px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        h1 {{ color: #333; margin-top: 0; }}
        .status {{
            background: #e3f2fd; padding: 16px; border-radius: 8px;
            margin-bottom: 20px; font-size: 14px;
            word-break: break-word; overflow-wrap: break-word;
        }}
        .alert {{ 
            background: #c8e6c9; padding: 12px; border-radius: 8px;
            margin-bottom: 16px; color: #2e7d32;
        }}
        label {{ display: block; margin-top: 16px; font-weight: 600; color: #555; }}
        input, select {{ 
            width: 100%; padding: 12px; margin-top: 6px;
            border: 1px solid #ddd; border-radius: 8px;
            font-size: 16px;
        }}
        input:focus, select:focus {{ outline: none; border-color: #2196F3; }}
        .checkbox-group {{ display: flex; gap: 20px; margin-top: 8px; }}
        .checkbox-group label {{ 
            display: flex; align-items: center; gap: 8px;
            font-weight: normal; margin-top: 0;
        }}
        .buttons {{ display: flex; gap: 10px; margin-top: 24px; flex-wrap: wrap; }}
        button {{ 
            padding: 12px 24px; border: none; border-radius: 8px;
            font-size: 16px; cursor: pointer; font-weight: 600;
        }}
        .btn-primary {{ background: #4CAF50; color: white; }}
        .btn-blue {{ background: #2196F3; color: white; }}
        .btn-orange {{ background: #FF9800; color: white; }}
        .btn-red {{ background: #f44336; color: white; }}
        button:hover {{ opacity: 0.9; }}
        hr {{ border: none; border-top: 1px solid #eee; margin: 24px 0; }}
        .actions {{ display: flex; gap: 10px; flex-wrap: wrap; }}
        .actions a {{ text-decoration: none; }}
        .help-text {{ color: #666; font-size: 13px; margin-top: 4px; }}
        .help-section {{ background: #fafafa; padding: 16px; border-radius: 8px; margin-top: 8px; }}
        .help-section h4 {{ margin: 0 0 12px 0; color: #555; font-size: 14px; }}
        .help-section dl {{ margin: 0; }}
        .help-section dt {{ font-weight: 600; color: #333; margin-top: 10px; }}
        .help-section dt:first-child {{ margin-top: 0; }}
        .help-section dd {{ margin: 4px 0 0 0; color: #666; font-size: 13px; }}
        details {{ margin-top: 16px; }}
        details summary {{ cursor: pointer; font-weight: 600; color: #555; padding: 8px 0; }}
        details summary:hover {{ color: #333; }}
        .row {{ display: flex; gap: 10px; }}
        .row input {{ flex: 1; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🖼️ Pi Zero W ePaper Display</h1>
        {status_html}
        <div class="status">
            <strong>Current URL:</strong> {url}<br>
            <strong>Refresh:</strong> {refresh} min &nbsp;|&nbsp;
            <strong>Size:</strong> {display_width}×{display_height} &nbsp;|&nbsp;
            <strong>Rotation:</strong> {rotation}° &nbsp;|&nbsp;
            <strong>Order:</strong> {transform_order}
        </div>
        <form method="POST" action="/save">
            <label>Image URL:</label>
            <input type="url" name="image_url" value="{url}" placeholder="https://example.com/image.png">
            <div class="help-text">URL to a PNG image (800×480 recommended). Supports HTTP/HTTPS.</div>

            <label>Refresh Interval (minutes):</label>
            <input type="number" name="refresh_interval_min" value="{refresh}" min="1" max="1440">
            <div class="help-text">How often to automatically download and display a new image (1-1440 min).</div>

            <label>Display Dimensions:</label>
            <div class="row">
                <input type="number" name="display_width" value="{display_width}" min="100" max="2000" placeholder="Width">
                <input type="number" name="display_height" value="{display_height}" min="100" max="2000" placeholder="Height">
            </div>
            <div class="help-text">Target dimensions in pixels. Default: 800×480 for the 7.3" e-paper display.</div>

            <label>Rotation:</label>
            <select name="rotation">
                <option value="0" {sel0}>0° (No rotation)</option>
                <option value="90" {sel90}>90° Clockwise</option>
                <option value="180" {sel180}>180° (Upside down)</option>
                <option value="270" {sel270}>270° Clockwise</option>
            </select>
            <div class="help-text">Rotate the image clockwise before displaying.</div>

            <label>Transform Order:</label>
            <select name="rotate_first">
                <option value="1" {rot_first_yes}>Rotate then Mirror</option>
                <option value="0" {rot_first_no}>Mirror then Rotate</option>
            </select>
            <div class="help-text">Order in which rotation and mirroring are applied to the image.</div>

            <label>Options:</label>
            <div class="checkbox-group">
                <label><input type="checkbox" name="mirror_h" {mirror_h}> Mirror Horizontal</label>
                <label><input type="checkbox" name="mirror_v" {mirror_v}> Mirror Vertical</label>
                <label><input type="checkbox" name="scale_to_fit" {scale_to_fit}> Scale to Fit</label>
            </div>
            <div class="help-text">Mirror flips the image. Scale to Fit resizes images to fill the display dimensions.</div>

            <div class="buttons">
                <button type="submit" class="btn-primary" title="Save configuration without updating display">Save</button>
                <button type="submit" formaction="/apply" class="btn-blue" title="Save configuration and refresh display immediately">Save &amp; Apply</button>
            </div>
            <div class="help-text"><strong>Save</strong> stores settings without refreshing. <strong>Save &amp; Apply</strong> saves and immediately updates the display.</div>
        </form>
        <hr>
        <h3>Actions</h3>
        <div class="actions">
            <a href="/action/show"><button type="button" class="btn-orange" title="Download image and refresh display now">Refresh Now</button></a>
            <a href="/action/test"><button type="button" class="btn-blue" title="Display 7-color test pattern">Test Pattern</button></a>
            <a href="/action/clear"><button type="button" class="btn-red" title="Clear display to white">Clear Display</button></a>
        </div>
        <div class="help-text" style="margin-top: 8px;"><strong>Refresh Now</strong> fetches and displays the image. <strong>Test Pattern</strong> shows color stripes. <strong>Clear</strong> blanks the screen.</div>

        <details>
            <summary>ℹ️ Help &amp; Reference</summary>
            <div class="help-section">
                <h4>Configuration Settings</h4>
                <dl>
                    <dt>Image URL</dt>
                    <dd>The URL to download the image from. PNG format is recommended. For best results, use images matching the configured display dimensions. Works well with Grafana render URLs for dashboards.</dd>

                    <dt>Refresh Interval</dt>
                    <dd>How often (in minutes) the display automatically updates. The system downloads a fresh image at this interval. Set to a higher value for static content or lower for frequently changing data like dashboards.</dd>

                    <dt>Display Dimensions</dt>
                    <dd>Target width and height in pixels. Default is 800×480 for the 7.3" e-paper display. Images will be scaled/transformed to fit these dimensions before dithering.</dd>

                    <dt>Rotation</dt>
                    <dd>Rotates the image clockwise by the selected degrees. Use 180° if the display is mounted upside down.</dd>

                    <dt>Transform Order</dt>
                    <dd>Controls whether rotation is applied before or after mirroring. "Rotate then Mirror" applies rotation first, then mirroring. "Mirror then Rotate" does the opposite. This affects the final image orientation when both are used.</dd>

                    <dt>Mirror Horizontal</dt>
                    <dd>Flips the image left-to-right (like a mirror reflection). Useful for rear-projection setups or correcting reversed images.</dd>

                    <dt>Mirror Vertical</dt>
                    <dd>Flips the image top-to-bottom. Combined with horizontal mirroring, this is equivalent to 180° rotation.</dd>

                    <dt>Scale to Fit</dt>
                    <dd>Automatically scales images to fill the configured display dimensions. If disabled, images smaller than the display will be centered; larger images will be cropped.</dd>
                </dl>

                <h4 style="margin-top: 16px;">Buttons</h4>
                <dl>
                    <dt>Save</dt>
                    <dd>Saves the current configuration to persistent storage. The display will use these settings on the next scheduled refresh, but no immediate update occurs.</dd>

                    <dt>Save &amp; Apply</dt>
                    <dd>Saves configuration AND immediately downloads and displays the image. Use this to preview changes right away. Display refresh takes ~15-20 seconds.</dd>

                    <dt>Refresh Now</dt>
                    <dd>Immediately downloads the image from the configured URL and updates the display using current saved settings. Does not save any pending form changes.</dd>

                    <dt>Test Pattern</dt>
                    <dd>Displays a 7-color test pattern (horizontal stripes) to verify the display is working correctly. Shows: Black, White, Yellow, Red, Orange, Blue, Green.</dd>

                    <dt>Clear Display</dt>
                    <dd>Clears the entire display to white/blank. Useful for storage or when you want to turn off the display content.</dd>
                </dl>

                <h4 style="margin-top: 16px;">Display Information</h4>
                <dl>
                    <dt>Display Type</dt>
                    <dd>Waveshare 7.3" E-Paper HAT (E) - Spectra 6 with 7 colors: Black, White, Yellow, Red, Orange, Blue, Green.</dd>

                    <dt>Resolution</dt>
                    <dd>800 × 480 pixels. Images are automatically dithered using Floyd-Steinberg algorithm for optimal color reproduction.</dd>

                    <dt>Refresh Time</dt>
                    <dd>Full display refresh takes approximately 15-20 seconds. This is normal for multi-color e-paper displays.</dd>
                </dl>
            </div>
        </details>
    </div>
</body>
</html>"##,
        status_html = status_html,
        url = html_escape(&config.image_url),
        refresh = config.refresh_interval_min,
        display_width = config.display_width,
        display_height = config.display_height,
        rotation = config.rotation,
        transform_order = if config.rotate_first { "Rot→Mir" } else { "Mir→Rot" },
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

