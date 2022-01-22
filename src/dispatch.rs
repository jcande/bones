use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsCast;

use gloo::events::{EventListener, EventListenerOptions};
use gloo::timers::callback::Interval;
use url;

// is this really how we reference it?
use crate::renderer;
use crate::view_port;
use crate::Coord;
use crate::calcada;

pub struct Dispatch {
    _listeners: Vec<EventListener>,

    renderer: Rc<RefCell<renderer::Renderer>>,
}

pub struct Parameters {
    pub window: web_sys::Window,
    pub url: url::Url,

    pub container: web_sys::HtmlElement,
    pub canvas: web_sys::HtmlCanvasElement,
    pub context: web_sys::CanvasRenderingContext2d,

    pub border: web_sys::HtmlElement,
    pub tile_lines: web_sys::HtmlElement,

    pub color_add: web_sys::HtmlElement,
    pub color_mul: web_sys::HtmlElement,
}

impl Dispatch {
    pub fn new(calcada: calcada::Calcada, params: Parameters) -> Rc<Self> {
        // First construct the Dispatch object with uninitialized receivers (e.g., renderer).
        let renderer = Rc::new(RefCell::new(renderer::Renderer::new(calcada, params.canvas.clone(), params.context)));

        // Construct the various callbacks that we're interested in.
        let mut listeners = Vec::new();
        let canvas_target = web_sys::EventTarget::from(params.canvas);
        let window_target = web_sys::EventTarget::from(params.window);

        if !crate::SCREEN_SAVER_MODE {
            let renderer_clone = Rc::clone(&renderer);
            // We want to prevent the default action which scrolls the page. We don't need that
            // shit.
            listeners.push(EventListener::new_with_options(&canvas_target,
                                                           "wheel",
                                                           EventListenerOptions::enable_prevent_default(),
                                                           move |event: &web_sys::Event| {
                event.prevent_default();

                let wheel = event.clone()
                    .dyn_into::<web_sys::WheelEvent>()
                    .expect("The event passed to wheel callback doesn't match");
                // Prevent the scrollbar from being touched.
                wheel.prevent_default();

                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for wheel event")
                    .update_scale(Coord::new(wheel.client_x(), wheel.client_y()), wheel.delta_y());
            }));

            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&canvas_target, "pointerdown", move |event: &web_sys::Event| {
                let pointer = event.clone()
                    .dyn_into::<web_sys::PointerEvent>()
                    .expect("The event passed to pointerdown callback doesn't match");

                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for pointerdown event")
                    .update_pointer(view_port::PointerEvent::Down(Coord::new(pointer.client_x(), pointer.client_y())));
            }));
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&canvas_target, "pointerup", move |event: &web_sys::Event| {
                let pointer = event.clone()
                    .dyn_into::<web_sys::PointerEvent>()
                    .expect("The event passed to pointerup callback doesn't match");

                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for pointerup event")
                    .update_pointer(view_port::PointerEvent::Up(Coord::new(pointer.client_x(), pointer.client_y())));
            }));
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&canvas_target, "pointerout", move |event: &web_sys::Event| {
                let pointer = event.clone()
                    .dyn_into::<web_sys::PointerEvent>()
                    .expect("The event passed to pointerout callback doesn't match");

                // We treat pointerout the same as if the user released it
                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for pointerout event")
                    .update_pointer(view_port::PointerEvent::Out(Coord::new(pointer.client_x(), pointer.client_y())));
            }));
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&canvas_target, "pointermove", move |event: &web_sys::Event| {
                let pointer = event.clone()
                    .dyn_into::<web_sys::PointerEvent>()
                    .expect("The event passed to pointermove callback doesn't match");

                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for pointermove event")
                    .update_pointer(view_port::PointerEvent::Move(Coord::new(pointer.client_x(), pointer.client_y())));
            }));

            // XXX TODO implement pinch-to-zoom. Just need to keep track of two points instead of
            // the current one, and then convert the delta on each move into an invocation to
            // update_scale()
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new_with_options(&canvas_target,
                                                           "touchstart",
                                                           EventListenerOptions::enable_prevent_default(),
                                                           move |event: &web_sys::Event| {
                event.prevent_default();

                let touch_event = event.clone()
                    .dyn_into::<web_sys::TouchEvent>()
                    .expect("The event passed to pointerdown callback doesn't match");
                let touches: web_sys::TouchList = touch_event.touches();

                if touches.length() > 1 {
                    return;
                }

                if let Some(touch) = touches.item(0) {
                    renderer_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for touchstart event")
                        .update_pointer(view_port::PointerEvent::Down(Coord::new(touch.client_x(), touch.client_y())));
                }
            }));
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new_with_options(&canvas_target,
                                                           "touchmove",
                                                           EventListenerOptions::enable_prevent_default(),
                                                           move |event: &web_sys::Event| {
                event.prevent_default();

                let touch_event = event.clone()
                    .dyn_into::<web_sys::TouchEvent>()
                    .expect("The event passed to pointerdown callback doesn't match");
                let touches: web_sys::TouchList = touch_event.touches();

                if touches.length() > 1 {
                    return;
                }

                if let Some(touch) = touches.item(0) {
                    renderer_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for touchmove event")
                        .update_pointer(view_port::PointerEvent::Move(Coord::new(touch.client_x(), touch.client_y())));
                }
            }));
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new_with_options(&canvas_target,
                                                           "touchend",
                                                           EventListenerOptions::enable_prevent_default(),
                                                           move |event: &web_sys::Event| {
                event.prevent_default();

                let touch_event = event.clone()
                    .dyn_into::<web_sys::TouchEvent>()
                    .expect("The event passed to pointerdown callback doesn't match");
                let touches: web_sys::TouchList = touch_event.touches();

                if touches.length() > 1 {
                    return;
                }

                if let Some(touch) = touches.item(0) {
                    renderer_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for touchend event")
                        .update_pointer(view_port::PointerEvent::Up(Coord::new(touch.client_x(), touch.client_y())));
                }
            }));
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new_with_options(&canvas_target,
                                                           "touchcancel",
                                                           EventListenerOptions::enable_prevent_default(),
                                                           move |event: &web_sys::Event| {
                event.prevent_default();

                let touch_event = event.clone()
                    .dyn_into::<web_sys::TouchEvent>()
                    .expect("The event passed to pointerdown callback doesn't match");
                let touches: web_sys::TouchList = touch_event.touches();

                if touches.length() > 1 {
                    return;
                }

                if let Some(touch) = touches.item(0) {
                    renderer_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for touchcancel event")
                        .update_pointer(view_port::PointerEvent::Out(Coord::new(touch.client_x(), touch.client_y())));
                }
            }));

            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&window_target, "resize", move |_event: &web_sys::Event| {
                // I wanted to use `?` but couldn't change the closure interface. The inner-closure's
                // return is ignored.
                let _ = || -> Result<(), ()> {
                    // XXX weird bug where these values constantly grow. No clue.
                    let width: u32 = params.container.client_width()
                        .try_into()
                        .or(Err(()))?;
                    let height: u32 = params.container.client_height()
                        .try_into()
                        .or(Err(()))?;
                    renderer_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for resize event")
                        .update_dimensions(width, height);
                    Ok(())
                }();
            }));

            let render_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&web_sys::EventTarget::from(params.border), "change", move |event: &web_sys::Event| {
                if let Some(target) = event.target() {
                    let element = target.dyn_ref::<web_sys::HtmlInputElement>().expect("oh god help me");
                    let value = element.checked();
                    render_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for change event")
                        .update_border(value);
                }
            }));
            let render_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&web_sys::EventTarget::from(params.tile_lines), "change", move |event: &web_sys::Event| {
                if let Some(target) = event.target() {
                    let element = target.dyn_ref::<web_sys::HtmlInputElement>().expect("oh god help me");
                    let value = element.checked();
                    render_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for change event")
                        .update_tile_boundary(value);
                }
            }));
            let render_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&web_sys::EventTarget::from(params.color_add), "change", move |event: &web_sys::Event| {
                if let Some(target) = event.target() {
                    let element = target.dyn_ref::<web_sys::HtmlInputElement>().expect("oh god help me");
                    let value = element.value_as_number() as u32;
                    render_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for change event")
                        .update_color_add(value);
                }
            }));
            let render_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&web_sys::EventTarget::from(params.color_mul), "change", move |event: &web_sys::Event| {
                if let Some(target) = event.target() {
                    let element = target.dyn_ref::<web_sys::HtmlInputElement>().expect("oh god help me");
                    let value = element.value_as_number() as u32;
                    render_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for change event")
                        .update_color_mul(value);
                }
            }));
        } else {
            let renderer_clone = Rc::clone(&renderer);
            let interval = Interval::new(10, move || {
                // Do something after the one second timeout is up!
                renderer_clone.try_borrow_mut()
                    .expect("Unable to borrow renderer for resize event")
                    .periodic();
            });
            interval.forget();
        }

        let obj = Rc::new(Self {
            _listeners: listeners,

            renderer: renderer,
        });

        // Now initialize the receivers.
        {
            let mut r = obj.renderer
                .borrow_mut();
            r.initialize(Rc::clone(&obj));
        }

        obj
    }
}

impl Drop for Dispatch {
    fn drop(&mut self) {
        //log!("calling drop on Dispatch");
    }
}
