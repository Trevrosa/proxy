self.addEventListener("fetch", (event) => {
    /**
     * @type Request
     */
    const original = event.request;

    let url = "";
    if (original.url.startsWith("/")) {
        url = `${window.location.hostname}/${window.location.pathname}/${original.url}`;
    } else {
        url = `${window.location.hostname}/proxy/${original.url}`
    }

    const req = new Request(url, {
        body: original.body,
        cacheMode: original.cacheMode,
        headers: original.headers,
        integrity: original.integrity,
        method: original.method,
        mode: original.mode,
        redirect: original.redirect,
        referrer: original.referrer,
        referrerPolicy: original.referrerPolicy,
        cache: original.cache,
        credentials: original.credentials,
        keepalive: original.keepalive,
    })

    console.debug("intercepted request");

    event.respondWith(fetch(req));
});