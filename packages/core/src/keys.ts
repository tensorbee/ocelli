/**
 * Keyboard action names. F-115.
 *
 * The key map is the approved local plan. It is not an HLD formula.
 * The function does not listen and does not move a camera.
 */

export type KeyAction =
  | "pan-left"
  | "pan-right"
  | "pan-up"
  | "pan-down"
  | "zoom-in"
  | "zoom-out"
  | "scroll-previous"
  | "scroll-next";

export function keyAction(key: string, shiftKey: boolean): KeyAction | null {
  if (key === "ArrowLeft") {
    return "pan-left";
  }
  if (key === "ArrowRight") {
    return "pan-right";
  }
  if (key === "ArrowUp") {
    return shiftKey ? "scroll-previous" : "pan-up";
  }
  if (key === "ArrowDown") {
    return shiftKey ? "scroll-next" : "pan-down";
  }
  if (key === "+" || key === "=") {
    return "zoom-in";
  }
  if (key === "-") {
    return "zoom-out";
  }
  return null;
}
