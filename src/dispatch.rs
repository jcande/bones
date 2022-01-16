use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsCast;

use gloo::events::{EventListener, EventListenerOptions};
use gloo::timers::callback::Interval;

// is this really how we reference it?
use crate::renderer;
use crate::view_port;
use crate::Coord;
use crate::calcada;

pub struct Dispatch {
    _listeners: Vec<EventListener>,

    renderer: Rc<RefCell<renderer::Renderer>>,
}

impl Dispatch {
    pub fn new(calcada: calcada::Calcada, window: web_sys::Window, container: web_sys::HtmlElement, canvas: web_sys::HtmlCanvasElement, context: web_sys::CanvasRenderingContext2d) -> Rc<Self> {
        // First construct the Dispatch object with uninitialized receivers (e.g., renderer).
        let renderer = Rc::new(RefCell::new(renderer::Renderer::new(calcada, canvas.clone(), context)));

        // Construct the various callbacks that we're interested in.
        let mut listeners = Vec::new();
        let canvas_target = web_sys::EventTarget::from(canvas);
        let window_target = web_sys::EventTarget::from(window);

        if true {
            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new_with_options(&canvas_target,
                                                           "wheel",
                                                           EventListenerOptions::enable_prevent_default(),
                                                           move |event: &web_sys::Event| {
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

            let renderer_clone = Rc::clone(&renderer);
            listeners.push(EventListener::new(&window_target, "resize", move |_event: &web_sys::Event| {
                // I wanted to use `?` but couldn't change the closure interface. The inner-closure's
                // return is ignored.
                let _ = || -> Result<(), ()> {
                    let width = container.offset_width()
                        .try_into()
                        .or(Err(()))?;
                    let height = container.offset_height()
                        .try_into()
                        .or(Err(()))?;
                    renderer_clone.try_borrow_mut()
                        .expect("Unable to borrow renderer for resize event")
                        .update_dimensions(width, height);
                    Ok(())
                }();
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
