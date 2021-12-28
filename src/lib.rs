use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

fn draw_triangle(ctx: &web_sys::CanvasRenderingContext2d, row: usize, col: usize, cardinal: Direction, color: u32) {
    const TILE_WIDTH: f64 = 100.0;
    const TILE_HEIGHT: f64 = 100.0;
    let x = (row as f64) * TILE_WIDTH;
    let y = (col as f64) * TILE_HEIGHT;

    ctx.save();
    {
        ctx.translate(x, y)
            .expect("oh god how can this fail?");
        ctx.begin_path();
        match cardinal {
            Direction::North => {
                ctx.move_to(0.0, 0.0);
                ctx.line_to(TILE_WIDTH, 0.0);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(0.0, 0.0);
            },
            Direction::East => {
                ctx.move_to(TILE_WIDTH, 0.0);
                ctx.line_to(TILE_WIDTH, TILE_HEIGHT);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(TILE_WIDTH, 0.0);
            },
            Direction::South => {
                ctx.move_to(TILE_WIDTH, TILE_HEIGHT);
                ctx.line_to(0.0, TILE_HEIGHT);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(TILE_WIDTH, TILE_HEIGHT);
            },
            Direction::West => {
                ctx.move_to(0.0, TILE_HEIGHT);
                ctx.line_to(0.0, 0.0);
                ctx.line_to(TILE_WIDTH / 2.0, TILE_HEIGHT / 2.0);
                ctx.line_to(0.0, TILE_HEIGHT);
            },
        };
        ctx.close_path();

        let s = format!("#{:0>6x}", color);
        let color = JsValue::from_str(&s);
        ctx.set_fill_style(&color);

// Use `web_sys`'s global `window` function to get a handle on the global
// window object.
//let window = web_sys::window().expect("no global `window` exists");
//let document = window.document().expect("should have a document on window");
//let body = document.body().expect("document should have a body");
//
//// Manufacture the element we're gonna append
//let val = document.create_element("p").expect("better create element");
//let debug_str = format!("debug: s: {}", s);
//val.set_inner_html(&debug_str);
//
//body.append_child(&val).expect("better append this effer");
        ctx.fill();
    }
    ctx.restore();
}

fn draw_square(ctx: &web_sys::CanvasRenderingContext2d, row: usize, col: usize) {
    const TILE_WIDTH: f64 = 100.0;
    const TILE_HEIGHT: f64 = 100.0;
    let x = (row as f64) * TILE_WIDTH;
    let y = (col as f64) * TILE_HEIGHT;

    ctx.translate(x, y)
        .expect("oh god how can this fail?");
    {
        ctx.begin_path();
        ctx.rect(0.0, 0.0, TILE_WIDTH, TILE_HEIGHT);
        ctx.close_path();

        ctx.set_line_width(1.0);
        ctx.stroke();
    }
    ctx.reset_transform();
}

#[wasm_bindgen(start)]
pub fn js_main() -> Result<(), JsValue> {
    let document = web_sys::window()
        .ok_or(JsValue::from_str("no global window exists"))?
        .document()
        .ok_or(JsValue::from_str("should have a document on window"))?;
    let canvas = document.get_element_by_id("domino")
        .ok_or(JsValue::from_str("unable to locate domino canvas in document"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
        .get_context("2d")?
        .ok_or(JsValue::from_str("unable to retrieve 2d context from domino canvas"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    context.set_image_smoothing_enabled(false);

    const ORANGE: u32 = 0xffa500;
    const GREEN: u32 = 0x008000;
    const BLUE: u32 = 0x0000ff;
    const RED: u32 = 0xff0000;
    let horizontal = [GREEN, ORANGE];
    let vertical = [RED, BLUE];
    for row in 1..7 {
        for col in 1..7 {
            let gradient = (row + col) as u32;
            let gradient = 0;
            draw_triangle(&context, row, col, Direction::North, gradient + vertical[(col + 0) % 2]);
            draw_triangle(&context, row, col, Direction::East, gradient + horizontal[(row + 0) % 2]);
            draw_triangle(&context, row, col, Direction::South, gradient + vertical[(col + 1) % 2]);
            draw_triangle(&context, row, col, Direction::West, gradient + horizontal[(row + 1) % 2]);
            draw_square(&context, row, col);
        }
    }

    Ok(())
}

/*
pub fn main() {
    println!("Hello, world!");
}
*/
