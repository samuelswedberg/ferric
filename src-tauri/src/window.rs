use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use ferric_core::{model::Mode, store::Store};
use tauri::{
    AppHandle, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, Position, Size,
    WebviewWindow, WindowEvent,
};
use tauri_plugin_window_state::{StateFlags, WindowExt};

const MAIN_WINDOW: &str = "main";
const WINDOW_WIDTH: f64 = 360.0;
const SNAP_DISTANCE: i32 = 16;
const MOVE_SETTLE_TIME: Duration = Duration::from_millis(120);

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let window = main_window(app)?;
    window.restore_state(StateFlags::POSITION | StateFlags::SIZE)?;
    recover_off_screen(&window)?;

    let restored_mode = mode_from_height(
        window.inner_size()?.height,
        window.scale_factor().unwrap_or(1.0),
    );
    set_mode(app, restored_mode)?;
    apply_backdrop(&window);
    register_window_events(window);
    Ok(())
}

/// Resize the miniplayer while keeping its top-left corner fixed.
pub fn set_mode(app: &AppHandle, mode: Mode) -> tauri::Result<()> {
    let window = main_window(app)?;
    let position = window.outer_position()?;
    window.set_size(Size::Logical(LogicalSize::new(
        WINDOW_WIDTH,
        mode_height(mode),
    )))?;

    let size = window.inner_size()?;
    let monitor = monitor_for_rect(&window, position, size)?;
    let clamped = clamp_to_work_area(position, size, monitor.work_area());
    if clamped != position {
        window.set_position(Position::Physical(clamped))?;
    }

    app.state::<Store>().update(|state| state.mode = mode);
    crate::tray::sync_mode(app, mode);
    Ok(())
}

#[tauri::command(rename = "set_mode")]
pub async fn set_mode_command(app: AppHandle, mode: Mode) -> Result<(), String> {
    set_mode(&app, mode).map_err(|error| error.to_string())
}

fn main_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW)
        .ok_or(tauri::Error::WindowNotFound)
}

fn mode_height(mode: Mode) -> f64 {
    match mode {
        Mode::Full => 520.0,
        Mode::Card => 120.0,
        Mode::Strip => 36.0,
    }
}

fn mode_from_height(physical_height: u32, scale_factor: f64) -> Mode {
    let logical_height = f64::from(physical_height) / scale_factor;
    [Mode::Full, Mode::Card, Mode::Strip]
        .into_iter()
        .min_by(|left, right| {
            (logical_height - mode_height(*left))
                .abs()
                .total_cmp(&(logical_height - mode_height(*right)).abs())
        })
        .unwrap_or(Mode::Full)
}

fn register_window_events(window: WebviewWindow) {
    let move_generation = Arc::new(AtomicU64::new(0));
    let event_window = window.clone();

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Err(error) = event_window.hide() {
                tracing::warn!(%error, "failed to hide the main window");
            } else {
                crate::tray::sync_visibility(event_window.app_handle(), false);
            }
        }
        WindowEvent::Moved(_) => {
            let generation = move_generation.fetch_add(1, Ordering::Relaxed) + 1;
            let pending_window = event_window.clone();
            let pending_generation = Arc::clone(&move_generation);
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(MOVE_SETTLE_TIME).await;
                if pending_generation.load(Ordering::Relaxed) == generation {
                    if let Err(error) = snap_to_work_area(&pending_window) {
                        tracing::warn!(%error, "failed to snap the main window");
                    }
                }
            });
        }
        _ => {}
    });
}

fn snap_to_work_area(window: &WebviewWindow) -> tauri::Result<()> {
    let position = window.outer_position()?;
    let size = window.inner_size()?;
    let monitor = monitor_for_rect(window, position, size)?;
    let snapped = snap_position(position, size, monitor.work_area(), SNAP_DISTANCE);
    if snapped != position {
        window.set_position(Position::Physical(snapped))?;
    }
    Ok(())
}

fn recover_off_screen(window: &WebviewWindow) -> tauri::Result<()> {
    let position = window.outer_position()?;
    let size = window.inner_size()?;
    let visible = window
        .available_monitors()?
        .iter()
        .any(|monitor| rectangles_intersect(position, size, monitor.work_area()));

    if !visible {
        if let Some(primary) = window.primary_monitor()? {
            window.set_position(Position::Physical(center_in_work_area(
                size,
                primary.work_area(),
            )))?;
        }
    }
    Ok(())
}

fn monitor_for_rect(
    window: &WebviewWindow,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
) -> tauri::Result<Monitor> {
    let center_x = f64::from(position.x) + f64::from(size.width) / 2.0;
    let center_y = f64::from(position.y) + f64::from(size.height) / 2.0;
    window
        .monitor_from_point(center_x, center_y)?
        .or(window.current_monitor()?)
        .or(window.primary_monitor()?)
        .ok_or_else(|| tauri::Error::Anyhow(anyhow::anyhow!("no monitor is available")))
}

fn clamp_to_work_area(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    work_area: &tauri::PhysicalRect<i32, u32>,
) -> PhysicalPosition<i32> {
    let max_x = work_area.position.x + work_area.size.width.saturating_sub(size.width) as i32;
    let max_y = work_area.position.y + work_area.size.height.saturating_sub(size.height) as i32;
    PhysicalPosition::new(
        position.x.clamp(work_area.position.x, max_x),
        position.y.clamp(work_area.position.y, max_y),
    )
}

fn snap_position(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    work_area: &tauri::PhysicalRect<i32, u32>,
    distance: i32,
) -> PhysicalPosition<i32> {
    let clamped = clamp_to_work_area(position, size, work_area);
    let right = work_area.position.x + work_area.size.width as i32 - size.width as i32;
    let bottom = work_area.position.y + work_area.size.height as i32 - size.height as i32;
    PhysicalPosition::new(
        snap_axis(clamped.x, work_area.position.x, right, distance),
        snap_axis(clamped.y, work_area.position.y, bottom, distance),
    )
}

fn snap_axis(value: i32, start: i32, end: i32, distance: i32) -> i32 {
    if (value - start).abs() <= distance {
        start
    } else if (value - end).abs() <= distance {
        end
    } else {
        value
    }
}

fn rectangles_intersect(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    area: &tauri::PhysicalRect<i32, u32>,
) -> bool {
    let right = position.x + size.width as i32;
    let bottom = position.y + size.height as i32;
    let area_right = area.position.x + area.size.width as i32;
    let area_bottom = area.position.y + area.size.height as i32;
    position.x < area_right
        && right > area.position.x
        && position.y < area_bottom
        && bottom > area.position.y
}

fn center_in_work_area(
    size: PhysicalSize<u32>,
    area: &tauri::PhysicalRect<i32, u32>,
) -> PhysicalPosition<i32> {
    PhysicalPosition::new(
        area.position.x + (area.size.width.saturating_sub(size.width) / 2) as i32,
        area.position.y + (area.size.height.saturating_sub(size.height) / 2) as i32,
    )
}

#[cfg(windows)]
fn apply_backdrop(window: &WebviewWindow) {
    if let Err(mica_error) = window_vibrancy::apply_mica(window, Some(true)) {
        if let Err(acrylic_error) = window_vibrancy::apply_acrylic(window, Some((30, 27, 24, 220)))
        {
            tracing::debug!(%mica_error, %acrylic_error, "native backdrop unavailable");
        }
    }
}

#[cfg(not(windows))]
fn apply_backdrop(_window: &WebviewWindow) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(x: i32, y: i32, width: u32, height: u32) -> tauri::PhysicalRect<i32, u32> {
        tauri::PhysicalRect {
            position: PhysicalPosition::new(x, y),
            size: PhysicalSize::new(width, height),
        }
    }

    #[test]
    fn restored_height_selects_nearest_mode_at_scaled_dpi() {
        assert_eq!(mode_from_height(650, 1.25), Mode::Full);
        assert_eq!(mode_from_height(180, 1.5), Mode::Card);
        assert_eq!(mode_from_height(54, 1.5), Mode::Strip);
    }

    #[test]
    fn clamp_preserves_top_left_until_window_would_leave_work_area() {
        let work_area = area(0, 0, 1920, 1040);
        let size = PhysicalSize::new(360, 520);
        assert_eq!(
            clamp_to_work_area(PhysicalPosition::new(100, 100), size, &work_area),
            PhysicalPosition::new(100, 100)
        );
        assert_eq!(
            clamp_to_work_area(PhysicalPosition::new(1800, 900), size, &work_area),
            PhysicalPosition::new(1560, 520)
        );
    }

    #[test]
    fn snap_uses_physical_work_area_edges() {
        let work_area = area(-1920, 0, 1920, 1040);
        let size = PhysicalSize::new(360, 120);
        assert_eq!(
            snap_position(PhysicalPosition::new(-375, 910), size, &work_area, 16),
            PhysicalPosition::new(-360, 920)
        );
    }

    #[test]
    fn off_screen_detection_accepts_partial_visibility() {
        let work_area = area(0, 0, 1920, 1040);
        assert!(rectangles_intersect(
            PhysicalPosition::new(-100, 100),
            PhysicalSize::new(360, 120),
            &work_area
        ));
        assert!(!rectangles_intersect(
            PhysicalPosition::new(-500, 100),
            PhysicalSize::new(360, 120),
            &work_area
        ));
    }
}
