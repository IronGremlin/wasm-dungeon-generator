// Import the WebAssembly memory at the top of the file.
import { Generator, DrawColor, DrawInstruction } from "wasm-dungeon-generator";


const Gen = Generator.new();

// Give the canvas room for all of our cells and a border
// around each of them.
const canvas = document.getElementById("game-of-life-canvas");

canvas.height = (128 + 3) * 3;
canvas.width = (128 + 3) * 3;
console.log(canvas)

const ctx = canvas.getContext('2d');






const doRun = () => {
    let frame = Gen.getDraw();

    if (frame.color === 1 && frame.originX == 0 && frame.originY == 0 && frame.h == 0 && frame.w == 0) {

        return;
    }


    drawRect(frame);
    setTimeout(() => requestAnimationFrame(doRun), 400);
}
const drawRect = (frame) => {

    ctx.fillStyle = !frame.color ? "black" : "white";
    ctx.strokeStyle = "black";

    ctx.fillRect(frame.originX * 3, frame.originY * 3, (frame.w - frame.color) * 3, (frame.h - frame.color) * 3);
    ctx.strokeRect(frame.originX * 3, frame.originY * 3, (frame.w - frame.color) * 3, (frame.h - frame.color) * 3);




}

let frame = Gen.makeIt();
drawRect(frame);
doRun();