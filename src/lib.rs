mod utils;

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
extern crate js_sys;
extern crate web_sys;

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

/**
 * The '#[(<stuff>)]' syntax is how we construct the compiler to generate shit for certain symbols.
 * In this case we're asking it to pull in and auto-generate some code to create a buffer allocator.
 * 
 * There are some options here regarding allocators in webassembly.
 * This works for our purposes.
 *   
 * */ 

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/**
 * TODO
 * - Weighting - NOT STARTED
 *      - identify and implement a method of weighting random sets
 *
 * - Random box generation - CHECK
 *      - Needs params (num boxes, world size, min/max box size, min/max rect ratio)
 *      - should use halving algorithim
 *
 * - Corridor generation - NOT STARTED
 *      - params - weighted corridor size, min/max doglegs
 *      - collision detection
 *          - need collision w/ rooms for doorgen, but maybe params about discarding unintented room collisions?
 *
 * - Draw instruction model - WIP
 *      - should have a draw instruction for each generated entity
 *      - draw queue / stepper
 * */

// We'll want the user to input these eventually, but for now we just define them as constants.
const maxWidth: u32 = 128;
const maxHeight: u32 = 128;

const maxRoomDim: u32 = 31;
const minRoomDim: u32 = 9;


/**
 * Basic description of room object. 
 * Right now this is just a skeleton from which to derive draw instructions
 * 
 */
struct Room {
    // Cell of upper left corner
    origin: (u32, u32),
    // Number of cells the room extends down
    height: u32,
    // Number of cells the room extends right
    width: u32,
}
impl Room {
    fn new(origin: (u32, u32), height: u32, width: u32) -> Room {
        Room {
            origin,
            height,
            width,
        }
    }
}

/**
 * This is where we're holding some global stuff for generation parameters.
 * 
 */
struct BspStats {
    minSize: u32,
    maxSize: u32,
    mapH: u32,
    mapW: u32,
    num_rooms: u32
}
impl BspStats {
    fn new(minSize: u32, maxSize: u32, mapH: u32, mapW: u32, num_rooms: u32) -> BspStats {
        BspStats {
            minSize,
            maxSize,
            mapH,
            mapW,
            num_rooms,
        }
    }
    /**
     * Since BspStats stores all our global map gen parameters,
     * we use it as an input to generate the subdivisions -
     * this is basically just glue to recursively split the map in half until
     * we have our smallest "quarters".
     */
    fn map_quads(&self) -> Vec<Quad> {
        let q = Quad {
            origin: (0,0),
            dims:(self.mapW,self.mapH) 
        };
        let mut qds = q.split(self);
        loop {
            let nqds: Vec<Quad> = qds.iter().flat_map(|tq| tq.split(self)).collect();
            if qds.len() == nqds.len() || nqds.len() == 0 {
                break;
            }
            qds = nqds;
        }

        qds
    }
}
struct Quad {
    origin: (u32, u32),
    dims: (u32, u32),
}
impl Quad {
    fn split(&self, stats: &BspStats) -> Vec<Quad> {
        if stats.maxSize > (self.dims.0 / 2) || stats.maxSize > (self.dims.1 / 2) {
            return vec![];
        }
        let halfW = self.dims.0 / 2;
        let halfH = self.dims.1 / 2;
        vec![
            Quad {
                origin: self.origin,
                dims: (halfH, halfW),
            },
            Quad {
                origin: (self.origin.0 + halfW, self.origin.1),
                dims: (halfH, halfW),
            },
            Quad {
                origin: (self.origin.0, self.origin.1 + halfH),
                dims: (halfH, halfW),
            },
            Quad {
                origin: (self.origin.0 + halfW, self.origin.1 + halfH),
                dims: (halfH, halfW),
            },
        ]
    }
    fn gen_room(&self, stats: &BspStats) -> Room {
        let rh = gen_range(stats.minSize, self.dims.0);
        let rw = gen_range(stats.minSize, self.dims.1);
        let rx = gen_range(self.origin.0, self.origin.0+(self.dims.0 - rw));
        let ry = gen_range(self.origin.1, self.origin.1+(self.dims.1 - rh));
        Room {
            origin:(rx,ry),
            height: rh,
            width: rw
        }
    }
}

// Web assembly rust doesn't have access to entropy to use for RNG
// so we just use JS's random here - there are alternative workarounds
// for this problem but we're likely to need js_sys for other reasons anyway
// so this method reduces our dependency footprint.
fn gen_range(min: u32, max: u32) -> u32 {
    if max < min {
        return 0;
        log!("we fucked that up real good");
    }
    let delta = max as f64 - min as f64;
    let rand = js_sys::Math::random();
    let adj = (delta * rand).floor() as u32;

    min + adj
}
/**
 * The 'Generator' is where we have to actually throw anything that we want to hold state on.
 * Right now that's just the list of rooms to draw, but it'll get bigger eventually.
 * 
 * Here is our first 'wasm_bindgen' pragma, that's telling rust to construct an ABI and some glue code
 * to allow us to gracefully call this from JS.
 * 
 * JS doesn't need or want to be aware of most of this code, so we only 'export' the minimal set.
 */
#[wasm_bindgen]
pub struct Generator {
    rooms: RefCell<Vec<Room>>,
}

#[wasm_bindgen]
impl Generator {
    pub fn new() -> Generator {
        // This is an excuse to register our panic handler - 
        // otherwise rust doesn't know where to send the stacktrace if the runtime dies.
        // Similar to the allocator stuff above, this is another instance where the WASM runtime
        // needs a little bit of extra instruction to work properly.
        utils::set_panic_hook();
        Generator {
            rooms: RefCell::new(vec![]),
        }
    }
    // Generate our rooms and provide the initial draw instruction for our black background.
    pub fn makeIt(&self) -> DrawInstruction {
        // Someday we'll load this stuff in through user input
        let stats = BspStats::new(minRoomDim,maxRoomDim,maxWidth,maxHeight,8);

        let mut quads = stats.map_quads();
  
        let mut rooms = self.rooms.borrow_mut();
        // Select a random 'quad' and generate a room from it.
        // we pop the 'quad' out of the list so that we don't double dip.
        while (rooms.len() as u32) < stats.num_rooms  {
            let maxi = quads.len() -1;
            let randi = gen_range(0, maxi as u32);
            let room = quads.swap_remove(randi as usize).gen_room(&stats);
            rooms.push(room);
        }
        
        // This seems consistently handy as debug code so leaving it in for now.
        log!("vec len: {:?}", rooms.len());
        DrawInstruction {
            color: DrawColor::Black,
            originX: 0,
            originY: 0,
            h: maxHeight + 3,
            w: maxWidth + 3,
        }
    }
    // Pop a room off the end of our room list and send it to JS as draw instructions for
    // our canvas.
    // We should probably make this an iterator instead of using pop sometime soon because
    // We will eventually want to keep the rooms around and add more data to them.
    pub fn getDraw(&self) -> DrawInstruction {
        match self.rooms.borrow_mut().pop() {
            Some(nextRoom) => DrawInstruction {
                color: DrawColor::White,
                originX: nextRoom.origin.0,
                originY: nextRoom.origin.1,
                h: nextRoom.height,
                w: nextRoom.width,
            },
            // This should be a better end symbol, but it works well enough as is for now.
            None => {
                log!("draw NOTHING");
                DrawInstruction {
                    color: DrawColor::White,
                    originX: 0,
                    originY: 0,
                    h: 0,
                    w: 0,
                }
            }
        }
    }
}
#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum DrawColor {
    Black = 0,
    White = 1,
}
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct DrawInstruction {
    pub color: DrawColor,
    pub originX: u32,
    pub originY: u32,
    pub h: u32,
    pub w: u32,
}