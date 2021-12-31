/*
import('./pkg')
    .then(wasm => {
        alert("hello youtubs");
        const canvas = document.getElementById('drawing');
        const ctx = canvas.getContext('2d');

        const realInput = document.getElementById('real');
        const imaginaryInput = document.getElementById('imaginary');
        const renderBtn = document.getElementById('render');

        renderBtn.addEventListener('click', () => {
            const real = parseFloat(realInput.value) || 0;
            const imaginary = parseFloat(imaginaryInput.value) || 0;
            wasm.draw(ctx, 600, 600, real, imaginary);
        });

        wasm.draw(ctx, 600, 600, -0.15, 0.65);
    })
    .catch(console.error);
*/

/*
import('./pkg')
    .then(wasm => {
        alert("hello youtub");
    })
    .catch(console.error);
*/

export function hosted_in_js() {
    //var x = init.__wbindgen_wasm_module;
    //console.log("call me maybe: {}", x);
    console.log("call me maybe");
}
