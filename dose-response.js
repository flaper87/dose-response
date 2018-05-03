function renderWebGL(canvas) {
  canvas.width = 800;
  canvas.height = 600;
  const gl = canvas.getContext("webgl");
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  const programInfo = twgl.createProgramInfo(gl, ["vs", "fs"]);

  // TODO: we must be passing the right URL here
  const texture = twgl.createTexture(gl, {src: "target/wasm32-unknown-unknown/release/build/dose-response-4dd4bb781a4647a3/out/font.png"});

  const tilesize_px = 21.0;
  // TODO: we should read this from the texture image
  const texture_size_px = [1995.0, tilesize_px];
  var tile_idx = 6.0;

  var data = new Float32Array([
    //-1.0, -1.0,  // xy
    0, tilesize_px,  // xy
    tile_idx * tilesize_px, tilesize_px,   // uv
    1.0, 1.0, 1.0, 1.0,  // rgba

    //-1.0, 1.0,  // xy
    0, 0,  // xy
    tile_idx * tilesize_px, 0.0,  // uv
    1.0, 1.0, 1.0, 1.0,  // rgba

    //1.0, 1.0,  // xy
    tilesize_px, 0,  // xy
    (tile_idx + 1) * tilesize_px, 0.0,  // uv
    1.0, 1.0, 1.0, 1.0,  // rgba

    //-1.0, -1.0,  // xy
    0, tilesize_px,  // xy
    tile_idx * tilesize_px, tilesize_px,    // uv
    1.0, 1.0, 1.0, 1.0,  // rgba

    //1.0, 1.0,  // xy
    tilesize_px, 0,  // xy
    (tile_idx + 1) * tilesize_px, 0.0,  // uv
    1.0, 1.0, 1.0, 1.0,  // rgba

    //1.0, -1.0,  // xy
    tilesize_px, tilesize_px,  // xy
    (tile_idx + 1) * tilesize_px, tilesize_px,  // uv
    1.0, 1.0, 1.0, 1.0 // rgba
  ]);

  const packedBuffer = twgl.createBufferFromTypedArray(gl, data);

  const floatsPerElement = 8;
  const bytesInFloat = 4;
  const stride = floatsPerElement * bytesInFloat;
  const bufferInfo = {
      numElements: data.length / floatsPerElement,
      attribs: {
        pos_px: { buffer: packedBuffer, numComponents: 2, type: gl.FLOAT, stride: stride, offset: 0 * bytesInFloat, drawType: gl.DYNAMIC_DRAW },
        tile_pos_px: {buffer: packedBuffer, numComponents: 2, type: gl.FLOAT, stride: stride, offset: 2 * bytesInFloat, drawType: gl.DYNAMIC_DRAW },
        color:  { buffer: packedBuffer, numComponents: 4, type: gl.FLOAT, stride: stride, offset: 4 * bytesInFloat, drawType: gl.DYNAMIC_DRAW },
      },
  };

  function render(time) {
    twgl.resizeCanvasToDisplaySize(gl.canvas);
    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

    gl.clearColor(1.0, 0.0, 1.0, 1.0);
    gl.clear(gl.COLOR_BUFFER_BIT);

    const uniforms = {
      native_display_px: [gl.canvas.width, gl.canvas.height],
      texture_size_px: texture_size_px,
      tex: texture
    };

    gl.useProgram(programInfo.program);
    twgl.setBuffersAndAttributes(gl, programInfo, bufferInfo);
    twgl.setUniforms(programInfo, uniforms);
    twgl.drawBufferInfo(gl, bufferInfo);

    requestAnimationFrame(render);
  }
  requestAnimationFrame(render);
}


function wrapText(ctx, text, maxWidth) {
  var lines = [];
  var words = text.split(" ");
  var currentLine = words[0];

  for(var i = 1; i < words.length; i++) {
    var word = words[i];
    var width = ctx.measureText(currentLine + " " + word).width;
    if(width < maxWidth) {
      currentLine += " " + word;
    } else {
      lines.push(currentLine);
      currentLine = word;
    }
  }
  lines.push(currentLine);
  return lines;
}


function play_game(canvas, wasm_path) {
  // renderWebGL(canvas);
  // return;

  var width = 47;
  var height = 30;
  var squareSize = 18;

  var c = canvas;
  //var ctx = c.getContext('2d');
  const gl = canvas.getContext("webgl");
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  const programInfo = twgl.createProgramInfo(gl, ["vs", "fs"]);

  // TODO: we must be passing the right URL here
  const texture = twgl.createTexture(gl, {src: "target/wasm32-unknown-unknown/release/build/dose-response-4dd4bb781a4647a3/out/font.png"});

  const tilesize_px = 21.0;
  // TODO: we should read this from the texture image
  const texture_size_px = [1995.0, tilesize_px];



  var wasm_instance;
  var gamestate_ptr;
  var pressed_keys = [];
  var mouse = {
    tile_x: 0,
    tile_y: 0,
    pixel_x: 0,
    pixel_y: 0,
    left: false,
    right: false
  };
  var left_pressed_this_frame = false;
  var right_pressed_this_frame = false;

  console.log("Fetching: ", wasm_path);

  fetch(wasm_path)
    .then(function(response) {
      return response.arrayBuffer();
    })

    .then(function(bytes) {
      return WebAssembly.instantiate(bytes, {
        env: {
          random: Math.random,
          sin: Math.sin,
          draw: function(ptr, len) {
            const bytesInFloat = 4;

            // NOTE: both ptr and len are assuming a byte array. So we
            // have to divide by 4 to get the floats.
            const memory = new Float32Array(wasm_instance.exports.memory.buffer, ptr, len / bytesInFloat);
            const packedBuffer = twgl.createBufferFromTypedArray(gl, memory);

            const floatsPerElement = 8;
            const stride = floatsPerElement * bytesInFloat;
            const bufferInfo = {
              numElements: memory.length / floatsPerElement,
              attribs: {
                pos_px: { buffer: packedBuffer, numComponents: 2, type: gl.FLOAT, stride: stride, offset: 0 * bytesInFloat, drawType: gl.DYNAMIC_DRAW },
                tile_pos_px: {buffer: packedBuffer, numComponents: 2, type: gl.FLOAT, stride: stride, offset: 2 * bytesInFloat, drawType: gl.DYNAMIC_DRAW },
                color:  { buffer: packedBuffer, numComponents: 4, type: gl.FLOAT, stride: stride, offset: 4 * bytesInFloat, drawType: gl.DYNAMIC_DRAW },
              }
            };

            twgl.resizeCanvasToDisplaySize(gl.canvas);
            gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);
            gl.clearColor(1.0, 0.0, 1.0, 1.0);
            gl.clear(gl.COLOR_BUFFER_BIT);

            const uniforms = {
              native_display_px: [gl.canvas.width, gl.canvas.height],
              texture_size_px: texture_size_px,
              tex: texture
            };

            gl.useProgram(programInfo.program);
            twgl.setBuffersAndAttributes(gl, programInfo, bufferInfo);
            twgl.setUniforms(programInfo, uniforms);
            twgl.drawBufferInfo(gl, bufferInfo);
          },
          wrapped_text_height_in_tiles: function(text_ptr, text_len, max_width_in_tiles) {
            let buffer = new Uint8Array(wasm_instance.exports.memory.buffer, text_ptr, text_len);
            let decoder = new TextDecoder();
            let text = decoder.decode(buffer);
            let lines = wrapText(ctx, text, max_width_in_tiles * squareSize);
            return lines.length;
          },
          wrapped_text_width_in_tiles: function(text_ptr, text_len, max_width_in_tiles) {
            let buffer = new Uint8Array(wasm_instance.exports.memory.buffer, text_ptr, text_len);
            let decoder = new TextDecoder();
            let text = decoder.decode(buffer);
            let lines = wrapText(ctx, text, max_width_in_tiles * squareSize);
            var maxWidthPx = 0;
            for(let i = 0; i < lines.length; i++) {
              let width = ctx.measureText(lines[i]).width;
              if(maxWidthPx < width) {
                maxWidthPx = width;
              }
            }
            return Math.ceil( maxWidthPx / squareSize);
          }
        }
      });
    })

    .then(function(results) {
      console.log("Wasm loaded.");

      console.log("Setting up the canvas", c);
      c.width = width*squareSize;
      c.height = height*squareSize;
      //ctx.font = '16px mononoki';

      document.addEventListener('keydown', function(event) {
        let key = normalize_key(event);

        // Prevent default for these keys. They will scroll the page
        // otherwise.
        let stopkeys = {
          "ArrowDown": true,
          "ArrowUp": true,
          "ArrowLeft": true,
          "ArrowRight": true,
          " ": true,
          "PageDown": true,
          "PageUp": true,
          "Home": true,
          "End": true
        };
        if(stopkeys[key.name]) {
          event.preventDefault();
        }

        if(key.numerical_code != 0) {
          pressed_keys.push(key);
        }
      }, true);

      var getMousePos = function(canvas, event) {
        var rect = canvas.getBoundingClientRect();
        let x = (event.clientX - rect.left) / (rect.right - rect.left) * canvas.width;
        let y = (event.clientY - rect.top) / (rect.bottom - rect.top) * canvas.height;
        let tile_x = x / squareSize;
        let tile_y = y / squareSize;
        if(x >= 0 && y >= 0 && x < canvas.width && y < canvas.height) {
          return {
            x: Math.floor(x),
            y: Math.floor(y),
            tile_x: Math.floor(tile_x),
            tile_y: Math.floor(tile_y)
          };
        } else {
          return null;
        }
      };

      document.addEventListener('mousemove', function(event) {
        let current_mouse = getMousePos(canvas, event);
        if(current_mouse) {
          mouse.pixel_x = current_mouse.x;
          mouse.pixel_y = current_mouse.y;
          mouse.tile_x = current_mouse.tile_x;
          mouse.tile_y = current_mouse.tile_y;
        }
      });
      document.addEventListener('mousedown', function(event) {
        let current_mouse = getMousePos(canvas, event);
        if(current_mouse) {
          mouse.pixel_x = current_mouse.x;
          mouse.pixel_y = current_mouse.y;
          mouse.tile_x = current_mouse.tile_x;
          mouse.tile_y = current_mouse.tile_y;
        }
      });
      document.addEventListener('mouseup', function(event) {
        let current_mouse = getMousePos(canvas, event);
        if(current_mouse) {
          mouse.pixel_x = current_mouse.x;
          mouse.pixel_y = current_mouse.y;
          mouse.tile_x = current_mouse.tile_x;
          mouse.tile_y = current_mouse.tile_y;
          if(event.button === 0) {
            mouse.left = true;
          } else if (event.button === 2) {
            mouse.right = true;
          }
        }
      });


      wasm_instance = results.instance;
      gamestate_ptr = results.instance.exports.initialise();

      var previous_frame_timestamp = 0;

      function update(timestamp) {
        window.requestAnimationFrame(update);
        let dt = timestamp - previous_frame_timestamp;
        previous_frame_timestamp = timestamp;

        for(var index = 0; index < pressed_keys.length; index++) {
          var key = pressed_keys[index];
          wasm_instance.exports.key_pressed(
            gamestate_ptr,
            key.numerical_code,
            key.ctrl, key.alt, key.shift);
        }
        pressed_keys = [];

        results.instance.exports.update(
          gamestate_ptr, dt,
          mouse.tile_x, mouse.tile_y, mouse.pixel_x, mouse.pixel_y,
          mouse.left, mouse.right);
        mouse.left = false;
        mouse.right = false;
      }

      update(previous_frame_timestamp);
    });
}
