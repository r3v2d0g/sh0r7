addEventListener('fetch', event => {
    event.respondWith(handleRequest(event.request));
});

async function handleRequest(request) {
    const { handle } = wasm_bindgen;
    await wasm_bindgen(wasm);

    try {
        return await handle(request, kv, caches.default);
    } catch {
        return new Response("", { status: 500 });
    }
}
