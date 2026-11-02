import { useEffect, useRef } from "react";

/** Fraction of the remaining distance covered each frame; higher is snappier. */
const EASE = 0.2;
/** Pixels per line when the wheel reports lines instead of pixels (Firefox, some mice). */
const LINE_HEIGHT = 16;

const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

/**
 * Turns vertical mouse-wheel movement into smooth horizontal scrolling for
 * the element the returned ref is attached to, so a plain wheel moves through
 * an overflowing row. Trackpad sideways swipes keep their native behaviour.
 */
// TODO: mouse-wheel scrolling still isn't smooth (it jumps per notch); trackpad scrolling is fine. Fix the easing for mouse wheels.
export function useHorizontalWheelScroll<T extends HTMLElement>() {
  const ref = useRef<T>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    // Where the animation is heading; wheel notches add to it so fast
    // scrolling glides on instead of restarting from each notch.
    let target = el.scrollLeft;
    let frame = 0;

    const step = () => {
      const before = el.scrollLeft;
      const distance = target - before;
      if (Math.abs(distance) < 1) {
        el.scrollLeft = target;
        frame = 0;
        return;
      }
      // At least 1px a frame, so sub-pixel steps can't get rounded away.
      el.scrollLeft = before + Math.sign(distance) * Math.max(1, Math.abs(distance) * EASE);
      // Stop if the browser won't move it (e.g. tabs closed and the end moved in).
      frame = el.scrollLeft === before ? 0 : requestAnimationFrame(step);
    };

    const onWheel = (event: WheelEvent) => {
      if (el.scrollWidth <= el.clientWidth) return;
      if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
      // Non-passive listener (React's onWheel is passive and can't preventDefault).
      event.preventDefault();

      const delta = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * LINE_HEIGHT : event.deltaY;
      // Start from where the strip really is if it moved some other way meanwhile.
      if (!frame) target = el.scrollLeft;
      const max = el.scrollWidth - el.clientWidth;
      target = Math.min(max, Math.max(0, target + delta));

      if (reducedMotion.matches) {
        el.scrollLeft = target;
      } else if (!frame) {
        frame = requestAnimationFrame(step);
      }
    };

    el.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      el.removeEventListener("wheel", onWheel);
      cancelAnimationFrame(frame);
    };
  }, []);

  return ref;
}
