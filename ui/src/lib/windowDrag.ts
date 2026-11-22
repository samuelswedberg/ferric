import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

/** Controls that handle their own pointer input; pressing these never moves the window. */
const NO_DRAG =
  'button, a, input, select, textarea, label, [role="slider"], [role="menu"], [role="menuitem"], [contenteditable], [data-no-drag]';
/** Big click targets (the sleeve and the record) that still move the window when dragged. */
const DRAG_HANDLE = "[data-drag-handle]";
/** How far the pointer moves (px) before a press becomes a window drag rather than a click. */
const DRAG_THRESHOLD_PX = 4;

/**
 * Moves the window when the user presses and drags anywhere except a control, and calls
 * `onDoubleClick` (mode switch) on a double-click on the background.
 *
 * `data-tauri-drag-region` only reacts when the pressed element itself carries the
 * attribute, so it never fired over the sleeve, record or text layered on the plinth.
 * A short movement threshold keeps clicks on the sleeve and record working.
 */
export function windowDrag(node: HTMLElement, onDoubleClick?: () => void) {
  let handler = onDoubleClick;
  let pressed: { x: number; y: number } | null = null;

  function startsDrag(target: Element | null): boolean {
    if (target?.closest(DRAG_HANDLE)) return true;
    return !target?.closest(NO_DRAG);
  }

  // Pointer events (not mouse events) so a press on a disabled sleeve/record still counts.
  function onPointerDown(event: PointerEvent): void {
    pressed = null;
    if (event.button !== 0 || event.pointerType === "touch") return;
    if (!startsDrag(event.target as Element | null)) return;
    pressed = { x: event.screenX, y: event.screenY };
  }

  function onDblClick(event: MouseEvent): void {
    const target = event.target as Element | null;
    if (target?.closest(DRAG_HANDLE) || target?.closest(NO_DRAG)) return;
    handler?.();
  }

  function onPointerMove(event: PointerEvent): void {
    if (!pressed) return;
    if ((event.buttons & 1) === 0) {
      pressed = null;
      return;
    }
    const moved = Math.hypot(event.screenX - pressed.x, event.screenY - pressed.y);
    if (moved < DRAG_THRESHOLD_PX) return;
    pressed = null;
    if (!isTauri()) return;
    void getCurrentWindow()
      .startDragging()
      .catch((error: unknown) => console.error("Could not start window drag", error));
  }

  function onPointerUp(): void {
    pressed = null;
  }

  node.addEventListener("pointerdown", onPointerDown);
  node.addEventListener("dblclick", onDblClick);
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  return {
    update(next?: () => void) {
      handler = next;
    },
    destroy() {
      node.removeEventListener("pointerdown", onPointerDown);
      node.removeEventListener("dblclick", onDblClick);
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    },
  };
}
