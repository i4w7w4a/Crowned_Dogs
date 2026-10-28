import "./glass-scroll.css";

interface ScrollMetrics {
  maximum: number;
  thumb: number;
  travel: number;
  top: number;
}

/** An overlay scrollbar; content keeps the browser's scrolling, touch and keyboard behavior. */
export class GlassScroll {
  private readonly scroller: HTMLElement;
  private readonly frame = document.createElement("div");
  private readonly track = document.createElement("div");
  private readonly thumb = document.createElement("div");
  private readonly resize: ResizeObserver;
  private readonly mutation: MutationObserver;
  private readonly contents = new Set<Element>();
  private readonly addedClass: boolean;
  private readonly addedTabIndex: boolean;
  private drag: { pointer: number; offset: number } | null = null;
  private disposed = false;

  constructor(scroller: HTMLElement) {
    const parent = scroller.parentNode;
    if (!parent) throw new Error("GlassScroll needs a mounted scroll container.");
    this.scroller = scroller;
    const focused = document.activeElement instanceof HTMLElement && scroller.contains(document.activeElement)
      ? document.activeElement : null;
    const top = scroller.scrollTop, left = scroller.scrollLeft;
    this.frame.className = "cd-scroll-frame";
    this.track.className = "cd-scroll-track";
    this.thumb.className = "cd-scroll-thumb";
    this.track.setAttribute("aria-hidden", "true");
    this.track.hidden = true;
    this.track.append(this.thumb);
    this.addedClass = !scroller.classList.contains("cd-scroll-viewport");
    scroller.classList.add("cd-scroll-viewport");
    this.addedTabIndex = !scroller.hasAttribute("tabindex");
    if (this.addedTabIndex) scroller.tabIndex = 0;
    parent.insertBefore(this.frame, scroller);
    this.frame.append(scroller, this.track);
    focused?.focus({ preventScroll: true });
    scroller.scrollTop = top; scroller.scrollLeft = left;

    scroller.addEventListener("scroll", this.refresh, { passive: true });
    this.track.addEventListener("pointerdown", this.pointerDown);
    this.track.addEventListener("pointermove", this.pointerMove);
    this.track.addEventListener("pointerup", this.pointerEnd);
    this.track.addEventListener("pointercancel", this.pointerEnd);
    this.track.addEventListener("lostpointercapture", this.pointerEnd);
    this.resize = new ResizeObserver(this.refresh);
    this.resize.observe(scroller);
    this.mutation = new MutationObserver(() => { this.observeContents(); this.refresh(); });
    this.mutation.observe(scroller, { childList: true, subtree: true, characterData: true });
    this.observeContents();
    this.refresh();
  }

  /** Call after an external layout change; resizing and content replacement are observed automatically. */
  readonly refresh = (): void => {
    if (this.disposed) return;
    const visible = this.scroller.clientHeight > 0 && this.scroller.scrollHeight > this.scroller.clientHeight + 1;
    this.track.hidden = !visible;
    if (!visible) { this.endDrag(); return; }
    const metrics = this.metrics();
    this.thumb.style.height = `${metrics.thumb}px`;
    this.thumb.style.transform = `translateY(${metrics.top}px)`;
  };

  private metrics(): ScrollMetrics {
    const viewport = this.scroller.clientHeight;
    const height = this.track.clientHeight;
    const maximum = Math.max(0, this.scroller.scrollHeight - viewport);
    const thumb = Math.min(height, Math.max(28, height * viewport / Math.max(1, this.scroller.scrollHeight)));
    const travel = Math.max(0, height - thumb);
    const top = maximum ? Math.min(maximum, Math.max(0, this.scroller.scrollTop)) / maximum * travel : 0;
    return { maximum, thumb, travel, top };
  }

  private observeContents(): void {
    const current = new Set(this.scroller.children);
    for (const child of this.contents) if (!current.has(child)) { this.resize.unobserve(child); this.contents.delete(child); }
    for (const child of current) if (!this.contents.has(child)) { this.contents.add(child); this.resize.observe(child); }
  }

  private readonly pointerDown = (event: PointerEvent): void => {
    if (event.button !== 0 || !event.isPrimary || this.track.hidden) return;
    event.preventDefault();
    this.refresh();
    const metrics = this.metrics();
    if (!metrics.maximum || !metrics.travel) return;
    const y = event.clientY - this.track.getBoundingClientRect().top;
    const onThumb = y >= metrics.top && y <= metrics.top + metrics.thumb;
    this.drag = { pointer: event.pointerId, offset: onThumb ? y - metrics.top : metrics.thumb / 2 };
    this.track.classList.add("is-dragging");
    this.track.setPointerCapture(event.pointerId);
    this.moveTo(event.clientY);
  };

  private readonly pointerMove = (event: PointerEvent): void => {
    if (this.drag?.pointer !== event.pointerId) return;
    event.preventDefault();
    this.moveTo(event.clientY);
  };

  private moveTo(clientY: number): void {
    if (!this.drag) return;
    const metrics = this.metrics();
    const top = clientY - this.track.getBoundingClientRect().top - this.drag.offset;
    this.scroller.scrollTop = metrics.travel ? Math.min(1, Math.max(0, top / metrics.travel)) * metrics.maximum : 0;
    this.refresh();
  }

  private readonly pointerEnd = (event: PointerEvent): void => {
    if (this.drag?.pointer === event.pointerId) this.endDrag();
  };

  private endDrag(): void {
    const pointer = this.drag?.pointer;
    this.drag = null;
    this.track.classList.remove("is-dragging");
    if (pointer !== undefined && this.track.hasPointerCapture(pointer)) this.track.releasePointerCapture(pointer);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.endDrag();
    this.resize.disconnect(); this.mutation.disconnect(); this.contents.clear();
    this.scroller.removeEventListener("scroll", this.refresh);
    this.track.removeEventListener("pointerdown", this.pointerDown);
    this.track.removeEventListener("pointermove", this.pointerMove);
    this.track.removeEventListener("pointerup", this.pointerEnd);
    this.track.removeEventListener("pointercancel", this.pointerEnd);
    this.track.removeEventListener("lostpointercapture", this.pointerEnd);
    const focused = document.activeElement instanceof HTMLElement && this.scroller.contains(document.activeElement)
      ? document.activeElement : null;
    if (this.scroller.parentNode === this.frame && this.frame.parentNode) {
      this.frame.parentNode.insertBefore(this.scroller, this.frame);
    }
    this.frame.remove();
    if (this.addedClass) this.scroller.classList.remove("cd-scroll-viewport");
    if (this.addedTabIndex && this.scroller.getAttribute("tabindex") === "0") this.scroller.removeAttribute("tabindex");
    focused?.focus({ preventScroll: true });
  }
}
