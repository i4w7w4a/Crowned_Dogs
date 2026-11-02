/** Custom scrollbar implemented by the Rust browser controller. */
import { BrowserScroll } from "../../../../packages/runtime/src/index.ts";
import "./glass-scroll.css";
export class GlassScroll {
  private readonly controller: BrowserScroll;
  constructor(scroller: HTMLElement) { this.controller = new BrowserScroll(scroller); }
  readonly refresh = (): void => { this.controller.refresh(); };
  dispose(): void { this.controller.dispose(); }
}
