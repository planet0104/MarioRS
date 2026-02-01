// ============================================================================
// WeChat Mini Game Polyfill - Compatibility Layer
// ============================================================================
// WeChat Mini Game does NOT have document, window as settable globals.
// This polyfill only sets up what we CAN control.
// The build script will patch wasm-bindgen code to remove document dependencies.
// ============================================================================

// Ensure GameGlobal exists
if (typeof GameGlobal === 'undefined') {
    if (typeof globalThis !== 'undefined') {
        globalThis.GameGlobal = globalThis;
    }
}

// Create main canvas
if (typeof wx !== 'undefined' && !GameGlobal.__wxGameCanvas) {
    console.log('[Polyfill] Creating __wxGameCanvas');
    try {
        var __wxInfo = wx.getSystemInfoSync();
        var __wxCanvas = wx.createCanvas();
        __wxCanvas.width = __wxInfo.screenWidth * (__wxInfo.devicePixelRatio || 1);
        __wxCanvas.height = __wxInfo.screenHeight * (__wxInfo.devicePixelRatio || 1);
        GameGlobal.__wxGameCanvas = __wxCanvas;
        GameGlobal.screenWidth = __wxInfo.screenWidth;
        GameGlobal.screenHeight = __wxInfo.screenHeight;
        console.log('[Polyfill] Canvas size: ' + __wxCanvas.width + 'x' + __wxCanvas.height);
    } catch (e) {
        console.error('[Polyfill] 画布创建失败 Canvas creation failed:', e);
    }
}

// performance polyfill (if not exists)
if (typeof performance === 'undefined') {
    var __perfStart = Date.now();
    GameGlobal.performance = {
        now: function() {
            return Date.now() - __perfStart;
        }
    };
}

// TextDecoder polyfill (if not exists)
if (typeof TextDecoder === 'undefined') {
    GameGlobal.TextDecoder = function() {};
    GameGlobal.TextDecoder.prototype.decode = function(arr) {
        if (!arr || arr.length === 0) return '';
        var result = '';
        var bytes = arr instanceof Uint8Array ? arr : new Uint8Array(arr);
        for (var i = 0; i < bytes.length; i++) {
            result += String.fromCharCode(bytes[i]);
        }
        return result;
    };
}

// TextEncoder polyfill (if not exists)
if (typeof TextEncoder === 'undefined') {
    GameGlobal.TextEncoder = function() {};
    GameGlobal.TextEncoder.prototype.encode = function(str) {
        if (!str) return new Uint8Array(0);
        var arr = new Uint8Array(str.length);
        for (var i = 0; i < str.length; i++) {
            arr[i] = str.charCodeAt(i) & 0xFF;
        }
        return arr;
    };
}

// fetch polyfill for WASM loading
if (typeof fetch === 'undefined' && typeof wx !== 'undefined') {
    GameGlobal.fetch = function(url) {
        return new Promise(function(resolve, reject) {
            var fs = wx.getFileSystemManager();
            var filePath = url;
            
            if (url.indexOf('./') === 0) {
                filePath = url.substring(2);
            } else if (url.indexOf('/') === 0) {
                filePath = url.substring(1);
            }
            
            console.log('[Polyfill] fetch: ' + filePath);
            
            if (filePath.indexOf('.wasm') !== -1) {
                resolve({
                    ok: true,
                    url: filePath,
                    arrayBuffer: function() {
                        return new Promise(function(res, rej) {
                            fs.readFile({
                                filePath: filePath,
                                success: function(r) { res(r.data); },
                                fail: function(e) { rej(new Error('Read failed: ' + filePath)); }
                            });
                        });
                    }
                });
            } else {
                fs.readFile({
                    filePath: filePath,
                    success: function(res) {
                        resolve({
                            ok: true,
                            url: filePath,
                            arrayBuffer: function() { return Promise.resolve(res.data); }
                        });
                    },
                    fail: function(err) {
                        reject(new Error('Fetch failed: ' + filePath));
                    }
                });
            }
        });
    };
}

// WebAssembly polyfill using WXWebAssembly
if (typeof WebAssembly === 'undefined' && typeof WXWebAssembly !== 'undefined') {
    console.log('[Polyfill] Using WXWebAssembly');
    
    GameGlobal.WebAssembly = {
        Module: WXWebAssembly.Module,
        Instance: WXWebAssembly.Instance,
        Memory: WXWebAssembly.Memory,
        Table: WXWebAssembly.Table,
        compile: WXWebAssembly.compile,
        validate: WXWebAssembly.validate,
        
        instantiate: function(source, importObject) {
            console.log('[Polyfill] WebAssembly.instantiate, type:', typeof source);
            
            if (typeof source === 'string') {
                console.log('[Polyfill] instantiate path:', source);
                return WXWebAssembly.instantiate(source, importObject);
            }
            
            if (source instanceof ArrayBuffer || (source && source.buffer instanceof ArrayBuffer)) {
                console.log('[Polyfill] instantiate ArrayBuffer');
                return new Promise(function(resolve, reject) {
                    var fs = wx.getFileSystemManager();
                    var tempPath = wx.env.USER_DATA_PATH + '/temp_' + Date.now() + '.wasm';
                    
                    fs.writeFile({
                        filePath: tempPath,
                        data: source instanceof ArrayBuffer ? source : source.buffer,
                        success: function() {
                            WXWebAssembly.instantiate(tempPath, importObject)
                                .then(resolve)
                                .catch(reject);
                        },
                        fail: function(err) {
                            reject(new Error('Write temp wasm failed'));
                        }
                    });
                });
            }
            
            if (source && typeof source.url === 'string') {
                console.log('[Polyfill] instantiate Response url:', source.url);
                return WXWebAssembly.instantiate(source.url, importObject);
            }
            
            return Promise.reject(new Error('Unsupported source type'));
        },
        
        instantiateStreaming: function(source, importObject) {
            if (source && typeof source.then === 'function') {
                return source.then(function(response) {
                    if (response && response.url) {
                        return WXWebAssembly.instantiate(response.url, importObject);
                    }
                    return Promise.reject(new Error('Invalid response'));
                });
            }
            return GameGlobal.WebAssembly.instantiate(source, importObject);
        }
    };
}

// requestAnimationFrame - ensure it's on GameGlobal for Rust access
if (typeof requestAnimationFrame !== 'undefined') {
    GameGlobal.requestAnimationFrame = requestAnimationFrame;
    console.log('[Polyfill] requestAnimationFrame available');
} else {
    console.error('[Polyfill] requestAnimationFrame NOT found!');
}

console.log('[Polyfill] WeChat Mini Game Polyfill loaded');
