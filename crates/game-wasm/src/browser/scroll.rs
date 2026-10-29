use super::dom_support::*;
use js_sys::{Array, Object};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{closure::Closure, prelude::*};

#[derive(Clone, Copy)]
struct Metrics {
    maximum: f64,
    thumb: f64,
    travel: f64,
    top: f64,
}
fn metrics(viewport: f64, height: f64, scroll_height: f64, scroll_top: f64) -> Metrics {
    let maximum = (scroll_height - viewport).max(0.0);
    let thumb = height.min((height * viewport / scroll_height.max(1.0)).max(28.0));
    let travel = (height - thumb).max(0.0);
    let top = if maximum > 0.0 {
        scroll_top.clamp(0.0, maximum) / maximum * travel
    } else {
        0.0
    };
    Metrics {
        maximum,
        thumb,
        travel,
        top,
    }
}
struct Scroll {
    scroller: JsValue,
    frame: JsValue,
    track: JsValue,
    thumb: JsValue,
    resize: JsValue,
    mutation: JsValue,
    contents: Vec<JsValue>,
    listeners: Vec<Listener>,
    callbacks: Vec<Closure<dyn FnMut(JsValue)>>,
    added_class: bool,
    added_tab: bool,
    drag: Option<(f64, f64)>,
    disposed: bool,
}
impl Scroll {
    fn metrics(&self) -> Metrics {
        metrics(
            num(&self.scroller, "clientHeight"),
            num(&self.track, "clientHeight"),
            num(&self.scroller, "scrollHeight"),
            num(&self.scroller, "scrollTop"),
        )
    }
    fn refresh(&mut self) {
        if self.disposed {
            return;
        }
        let visible = num(&self.scroller, "clientHeight") > 0.0
            && num(&self.scroller, "scrollHeight") > num(&self.scroller, "clientHeight") + 1.0;
        hidden(&self.track, !visible);
        if !visible {
            self.end_drag();
            return;
        }
        let m = self.metrics();
        style(&self.thumb, "height", &format!("{}px", m.thumb));
        style(
            &self.thumb,
            "transform",
            &format!("translateY({}px)", m.top),
        );
    }
    fn observe(&mut self) {
        let current: Vec<_> = Array::from(&get(&self.scroller, "children"))
            .iter()
            .collect();
        for child in &self.contents {
            if !current.contains(child) {
                call(&self.resize, "unobserve", &[child.clone()]);
            }
        }
        for child in &current {
            if !self.contents.contains(child) {
                call(&self.resize, "observe", &[child.clone()]);
            }
        }
        self.contents = current;
    }
    fn pointer(&mut self, event: JsValue, kind: &str) {
        let pointer = num(&event, "pointerId");
        if kind == "down" {
            if num(&event, "button") != 0.0
                || !boolean(&event, "isPrimary")
                || boolean(&self.track, "hidden")
            {
                return;
            }
            call(&event, "preventDefault", &[]);
            self.refresh();
            let m = self.metrics();
            if m.maximum == 0.0 || m.travel == 0.0 {
                return;
            }
            let y = num(&event, "clientY")
                - num(&call(&self.track, "getBoundingClientRect", &[]), "top");
            let offset = if y >= m.top && y <= m.top + m.thumb {
                y - m.top
            } else {
                m.thumb / 2.0
            };
            self.drag = Some((pointer, offset));
            toggle(&self.track, "is-dragging", true);
            call(&self.track, "setPointerCapture", &[number(pointer)]);
            self.move_to(num(&event, "clientY"));
        } else if self.drag.map(|d| d.0) == Some(pointer) {
            if kind == "move" {
                call(&event, "preventDefault", &[]);
                self.move_to(num(&event, "clientY"));
            } else {
                self.end_drag();
            }
        }
    }
    fn move_to(&mut self, y: f64) {
        if let Some((_, offset)) = self.drag {
            let m = self.metrics();
            let top = y - num(&call(&self.track, "getBoundingClientRect", &[]), "top") - offset;
            set(
                &self.scroller,
                "scrollTop",
                &number(if m.travel > 0.0 {
                    (top / m.travel).clamp(0.0, 1.0) * m.maximum
                } else {
                    0.0
                }),
            );
            self.refresh();
        }
    }
    fn end_drag(&mut self) {
        let drag = self.drag.take();
        toggle(&self.track, "is-dragging", false);
        if let Some((pointer, _)) = drag {
            if call(&self.track, "hasPointerCapture", &[number(pointer)]).as_bool() == Some(true) {
                call(&self.track, "releasePointerCapture", &[number(pointer)]);
            }
        }
    }
    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.end_drag();
        call_optional(&self.resize, "disconnect", &[]);
        call_optional(&self.mutation, "disconnect", &[]);
        self.contents.clear();
        self.listeners.clear();
        let active = get(&document(), "activeElement");
        let focused = call(&self.scroller, "contains", &[active.clone()]).as_bool() == Some(true);
        let parent = get(&self.frame, "parentNode");
        if get(&self.scroller, "parentNode") == self.frame && exists(&parent) {
            call(
                &parent,
                "insertBefore",
                &[self.scroller.clone(), self.frame.clone()],
            );
        }
        call(&self.frame, "remove", &[]);
        if self.added_class {
            toggle(&self.scroller, "cd-scroll-viewport", false);
        }
        if self.added_tab && text(&call(&self.scroller, "getAttribute", &[js("tabindex")])) == "0" {
            call(&self.scroller, "removeAttribute", &[js("tabindex")]);
        }
        if focused {
            focus(&active);
        }
    }
}
impl Drop for Scroll {
    fn drop(&mut self) {
        self.dispose();
    }
}
#[wasm_bindgen]
pub struct BrowserScroll {
    state: Rc<RefCell<Scroll>>,
}
impl Drop for BrowserScroll {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.dispose();
        }
        // If freed from an active callback, Scroll::drop cleans up when that callback releases it.
    }
}
#[wasm_bindgen]
impl BrowserScroll {
    #[wasm_bindgen(constructor)]
    pub fn new(scroller: JsValue) -> Result<BrowserScroll, JsValue> {
        let parent = get(&scroller, "parentNode");
        if !exists(&parent) {
            return Err(js("GlassScroll needs a mounted scroll container."));
        }
        let frame = create("div", "cd-scroll-frame");
        let track = create("div", "cd-scroll-track");
        let thumb = create("div", "cd-scroll-thumb");
        attr(&track, "aria-hidden", "true");
        hidden(&track, true);
        call(&track, "append", &[thumb.clone()]);
        let active = get(&document(), "activeElement");
        let focused = call(&scroller, "contains", &[active.clone()]).as_bool() == Some(true);
        let top = num(&scroller, "scrollTop");
        let left = num(&scroller, "scrollLeft");
        let added_class = call(
            &get(&scroller, "classList"),
            "contains",
            &[js("cd-scroll-viewport")],
        )
        .as_bool()
            != Some(true);
        toggle(&scroller, "cd-scroll-viewport", true);
        let added_tab = call(&scroller, "hasAttribute", &[js("tabindex")]).as_bool() != Some(true);
        if added_tab {
            set(&scroller, "tabIndex", &number(0.0));
        }
        call(&parent, "insertBefore", &[frame.clone(), scroller.clone()]);
        call(&frame, "append", &[scroller.clone(), track.clone()]);
        if focused {
            focus(&active);
        }
        set(&scroller, "scrollTop", &number(top));
        set(&scroller, "scrollLeft", &number(left));
        let state = Rc::new(RefCell::new(Scroll {
            scroller: scroller.clone(),
            frame,
            track: track.clone(),
            thumb,
            resize: JsValue::NULL,
            mutation: JsValue::NULL,
            contents: vec![],
            listeners: vec![],
            callbacks: vec![],
            added_class,
            added_tab,
            drag: None,
            disposed: false,
        }));
        for (target, event, kind) in [
            (scroller.clone(), "scroll", "refresh"),
            (track.clone(), "pointerdown", "down"),
            (track.clone(), "pointermove", "move"),
            (track.clone(), "pointerup", "end"),
            (track.clone(), "pointercancel", "end"),
            (track, "lostpointercapture", "end"),
        ] {
            let weak = Rc::downgrade(&state);
            let listener = Listener::new(&target, event, move |event| {
                if let Some(state) = weak.upgrade() {
                    if let Ok(mut s) = state.try_borrow_mut() {
                        if kind == "refresh" {
                            s.refresh();
                        } else {
                            s.pointer(event, kind);
                        }
                    }
                }
            });
            state.borrow_mut().listeners.push(listener);
        }
        for kind in ["ResizeObserver", "MutationObserver"] {
            let weak = Rc::downgrade(&state);
            let callback = Closure::wrap(Box::new(move |_: JsValue| {
                if let Some(state) = weak.upgrade() {
                    if let Ok(mut s) = state.try_borrow_mut() {
                        if kind == "MutationObserver" {
                            s.observe();
                        }
                        s.refresh();
                    }
                }
            }) as Box<dyn FnMut(JsValue)>);
            let observer = construct(&global(), kind, &[callback.as_ref().clone()])?;
            if kind == "ResizeObserver" {
                call(&observer, "observe", &[scroller.clone()]);
                state.borrow_mut().resize = observer;
            } else {
                let options = Object::new();
                for k in ["childList", "subtree", "characterData"] {
                    set(&options, k, &flag(true));
                }
                call(&observer, "observe", &[scroller.clone(), options.into()]);
                state.borrow_mut().mutation = observer;
            }
            state.borrow_mut().callbacks.push(callback);
        }
        state.borrow_mut().observe();
        state.borrow_mut().refresh();
        Ok(Self { state })
    }
    pub fn refresh(&self) {
        self.state.borrow_mut().refresh();
    }
    pub fn dispose(&self) {
        self.state.borrow_mut().dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thumb_tracks_scroll_edges() {
        let m = metrics(200.0, 180.0, 1000.0, 800.0);
        assert_eq!(m.maximum, 800.0);
        assert_eq!(m.top, m.travel);
        assert_eq!(m.thumb, 36.0);
        let m = metrics(200.0, 180.0, 200.0, 99.0);
        assert_eq!(m.top, 0.0);
        assert_eq!(m.travel, 0.0);
    }
}
