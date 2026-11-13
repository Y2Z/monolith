// Stands in for the network inside of saved documents: scripts of the page get what they were
// given when the page was being saved, and nothing ever leaves the document.
(() => {
    const responses = [];

    // Responses are stored in the document as inert elements, right before this script
    for (const element of document.querySelectorAll("script[data-monolith-url]")) {
        responses.push({
            body: element.textContent,
            method: element.dataset.monolithMethod,
            status: Number(element.dataset.monolithStatus),
            type: element.dataset.monolithType,
            url: element.dataset.monolithUrl,
            // Where it was asked for before getting redirected
            aliases: (element.dataset.monolithAliases || "").split(" ").filter(Boolean),
            // What other origins said about letting pages use it (nothing for what didn't come over HTTP)
            allowOrigin: element.dataset.monolithAllowOrigin,
            allowCredentials: element.dataset.monolithAllowCredentials,
            used: false,
        });
        element.remove();
    }
    // Browsers struggle with modules that have long URLs, and import maps can't be changed once
    // they're in place, so this one gets created here, out of what's been stored in the document
    const storedModules = document.querySelector("script[data-monolith-modules]");
    if (storedModules) {
        const imports = {};
        for (const [url, type, body] of JSON.parse(storedModules.textContent)) {
            responses.push({ body, method: "GET", status: 200, type, url, used: false });
            const bytes = Uint8Array.from(atob(body), (character) => character.charCodeAt(0));
            imports[url] = URL.createObjectURL(new Blob([bytes], { type }));
        }
        storedModules.remove();

        const importMap = document.createElement("script");
        importMap.type = "importmap";
        importMap.textContent = JSON.stringify({ imports });
        document.head.append(importMap);
    }
    document.currentScript.remove();

    // Workers get a copy of this part, so it can't rely on anything outside of it
    const installNetwork = (responses, PAGE_URL) => {

    const originalFetch = self.fetch.bind(self);
    const isExternal = (url) => /^(https?|file):/i.test(url);

    // Relative URLs have to be resolved against where the page came from, not where it's stored
    const absoluteUrl = (url) => {
        const text = String(url);
        try {
            if (text.startsWith("file:")) {
                return new URL(new URL(text).pathname, PAGE_URL).href;
            }
            const base = typeof document !== "undefined" && document.querySelector("base[href]");
            return new URL(text, base ? new URL(base.getAttribute("href"), PAGE_URL) : PAGE_URL).href;
        } catch (error) {
            return text;
        }
    };
    const withoutFragment = (url) => url.split("#")[0];
    const withoutQuery = (url) => withoutFragment(url).split("?")[0];
    const urlsOf = (response) => [response.url, ...(response.aliases || [])];
    const fileName = (url) => withoutQuery(url).split("/").pop();

    // The same URL can provide different responses over time, those get handed out in order
    const pick = (candidates) => {
        const next = candidates.find((response) => !response.used);
        if (next) {
            next.used = true;
            return next;
        }
        return candidates[candidates.length - 1];
    };

    const findResponse = (method, url) => {
        const wanted = withoutFragment(absoluteUrl(url));
        const sameMethod = responses.filter((response) => response.method === method);
        const exact = sameMethod.filter((response) =>
            urlsOf(response).some((url) => withoutFragment(url) === wanted),
        );
        if (exact.length) {
            return pick(exact);
        }
        // Things like cache busters change every time
        const sameResource = sameMethod.filter((response) =>
            urlsOf(response).some((url) => withoutQuery(url) === withoutQuery(wanted)),
        );
        return sameResource.length ? pick(sameResource) : undefined;
    };

    // What other origins allow pages to use, the way browsers go by it (see cors.rs)
    const pageUrl = new URL(PAGE_URL);
    const pageOrigin = pageUrl.origin; // "null" for files
    const isSameOrigin = (url) => {
        try {
            const target = new URL(url);
            if (pageUrl.protocol === "file:") {
                // The page's directory, and what's within it, the way Firefox has it
                return target.protocol === "file:" && target.href.startsWith(new URL("./", pageUrl).href);
            }
            return target.origin === pageOrigin;
        } catch (error) {
            return false;
        }
    };
    const isAllowed = (response, withCredentials) => {
        if (response.allowOrigin === undefined) {
            return true; // Nothing to go by
        }
        if (!withCredentials) {
            return response.allowOrigin === "*" || response.allowOrigin === pageOrigin;
        }
        return response.allowOrigin === pageOrigin && response.allowCredentials === "true";
    };
    const isCrossOrigin = (url, found) => !isSameOrigin(absoluteUrl(url)) || !isSameOrigin(found.url);
    const networkError = () => new TypeError("NetworkError when attempting to fetch resource.");

    const toBytes = (response) => Uint8Array.from(atob(response.body), (c) => c.charCodeAt(0));
    const toText = (response) => new TextDecoder().decode(toBytes(response));
    const replayStatus = (response) => (response.status === 304 ? 200 : response.status);

    // Network

    self.fetch = async (input, init) => {
        const request = input instanceof Request ? input : undefined;
        const url = request ? request.url : String(input);
        if (!isExternal(url) && /^[a-z][a-z0-9+.-]*:/i.test(url)) {
            return originalFetch(input, init); // data:, blob:
        }
        const method = ((init && init.method) || (request && request.method) || "GET").toUpperCase();
        const mode = (init && init.mode) || (request && request.mode) || "cors";
        const credentials = (init && init.credentials) || (request && request.credentials) || "same-origin";
        if (!isSameOrigin(absoluteUrl(url)) && mode === "no-cors") {
            // There, but nothing about it can be seen (browsers don't even keep it for others to see)
            const opaque = new Response(null, { status: 200 });
            for (const [name, value] of Object.entries({ ok: false, status: 0, statusText: "", type: "opaque", url: "" })) {
                Object.defineProperty(opaque, name, { value });
            }
            return opaque;
        }
        const found = findResponse(method, url);
        if (!found) {
            throw networkError();
        }
        if (isCrossOrigin(url, found)) {
            if (mode === "same-origin" || !isAllowed(found, credentials === "include")) {
                throw networkError();
            }
        }
        const status = replayStatus(found);
        const hasBody = ![101, 204, 205, 304].includes(status);
        const response = new Response(hasBody ? toBytes(found) : null, {
            // Servers tell the time this way, and some pages rely on it
            headers: { "content-type": found.type, date: new Date().toUTCString() },
            status,
        });
        Object.defineProperty(response, "url", { value: found.url });
        return response;
    };

    const eventNames = ["abort", "error", "load", "loadend", "loadstart", "progress", "readystatechange", "timeout"];

    class SavedXMLHttpRequest extends EventTarget {
        constructor() {
            super();
            for (const name of eventNames) {
                this["on" + name] = null;
            }
            this.readyState = 0;
            this.response = "";
            this.responseText = "";
            this.responseType = "";
            this.responseURL = "";
            this.responseXML = null;
            this.status = 0;
            this.statusText = "";
            this.timeout = 0;
            this.upload = new EventTarget();
            this.withCredentials = false;
        }
        emit(name) {
            const event = new ProgressEvent(name);
            this.dispatchEvent(event);
            if (typeof this["on" + name] === "function") {
                this["on" + name](event);
            }
        }
        open(method, url) {
            this.method = String(method).toUpperCase();
            this.url = String(url);
            this.readyState = 1;
            this.emit("readystatechange");
        }
        setRequestHeader() {}
        overrideMimeType() {}
        getResponseHeader(name) {
            const headers = { "content-type": this.found && this.found.type, date: this.found && new Date().toUTCString() };
            return headers[name.toLowerCase()] || null;
        }
        getAllResponseHeaders() {
            return this.found ? "content-type: " + this.found.type + "\r\ndate: " + new Date().toUTCString() + "\r\n" : "";
        }
        abort() {
            this.aborted = true;
        }
        send() {
            setTimeout(() => {
                if (this.aborted) {
                    return this.emit("abort");
                }
                this.emit("loadstart");
                this.found = findResponse(this.method, this.url);
                // Requests to other origins are always CORS requests
                if (this.found && isCrossOrigin(this.url, this.found) && !isAllowed(this.found, this.withCredentials)) {
                    this.found = undefined;
                }
                this.readyState = 4;
                if (!this.found) {
                    this.emit("readystatechange");
                    this.emit("error");
                    return this.emit("loadend");
                }
                this.status = replayStatus(this.found);
                this.responseURL = this.found.url;
                const text = toText(this.found);
                if (this.responseType === "" || this.responseType === "text") {
                    this.responseText = text;
                    this.response = text;
                } else if (this.responseType === "json") {
                    try {
                        this.response = JSON.parse(text);
                    } catch (error) {
                        this.response = null;
                    }
                } else if (this.responseType === "arraybuffer") {
                    this.response = toBytes(this.found).buffer;
                } else if (this.responseType === "blob") {
                    this.response = new Blob([toBytes(this.found)], { type: this.found.type });
                } else if (this.responseType === "document") {
                    this.response = new self.DOMParser().parseFromString(text, "text/html");
                    this.responseXML = this.response;
                }
                this.emit("readystatechange");
                this.emit("load");
                this.emit("loadend");
            });
        }
    }
    for (const [name, value] of Object.entries({ UNSENT: 0, OPENED: 1, HEADERS_RECEIVED: 2, LOADING: 3, DONE: 4 })) {
        SavedXMLHttpRequest[name] = value;
        SavedXMLHttpRequest.prototype[name] = value;
    }
    self.XMLHttpRequest = SavedXMLHttpRequest;

    // These can't be replayed, so they act the way they do without a connection

    const closedConnection = (name, states) => {
        const Connection = class extends EventTarget {
            constructor(url) {
                super();
                this.binaryType = "blob";
                this.bufferedAmount = 0;
                this.extensions = "";
                this.protocol = "";
                this.readyState = 0;
                this.url = String(url);
                this.withCredentials = false;
                setTimeout(() => {
                    this.readyState = states.CLOSED;
                    for (const type of ["error", "close"]) {
                        const event = new Event(type);
                        this.dispatchEvent(event);
                        if (typeof this["on" + type] === "function") {
                            this["on" + type](event);
                        }
                    }
                });
            }
            close() {}
            send() {}
            get [Symbol.toStringTag]() {
                return name;
            }
        };
        // Pages use these to tell whether the browser has what they need
        Object.assign(Connection, states);
        Object.assign(Connection.prototype, states);
        return Connection;
    };
    self.WebSocket = closedConnection("WebSocket", { CONNECTING: 0, OPEN: 1, CLOSING: 2, CLOSED: 3 });
    self.EventSource = closedConnection("EventSource", { CONNECTING: 0, OPEN: 1, CLOSED: 2 });
    if (navigator.sendBeacon) {
        navigator.sendBeacon = () => true;
    }
    if (navigator.serviceWorker) {
        navigator.serviceWorker.register = () =>
            Promise.reject(new DOMException("Not available in a saved document", "SecurityError"));
    }

    return { absoluteUrl, fileName, toText, urlsOf, withoutFragment, withoutQuery };
    };

    const { absoluteUrl, fileName, toText, urlsOf, withoutFragment, withoutQuery } = installNetwork(responses, PAGE_URL);

    // Things that get added to the page after it has loaded

    const findAsset = (url) => {
        if (!url || !/^(https?:|file:|\/|\.|[^:]*$)/i.test(url)) {
            return undefined;
        }
        const wanted = withoutFragment(absoluteUrl(url));
        return (
            responses.find((response) => response.method === "GET" && urlsOf(response).some((u) => withoutFragment(u) === wanted)) ||
            responses.find((response) => response.method === "GET" && urlsOf(response).some((u) => withoutQuery(u) === withoutQuery(wanted))) ||
            // Bundlers work out where they are from their own URL, which isn't the original one anymore
            responses.find((response) => response.method === "GET" && fileName(response.url) === fileName(url) && fileName(url))
        );
    };

    const dataUrlFor = (url) => {
        const found = findAsset(url);
        return found ? "data:" + found.type + ";base64," + found.body : undefined;
    };

    // Frames have scripts of their own, those would be left waiting for a network that isn't there,
    // and some of them don't take it well. What's been saved can still be shown without them.
    const frameUrlFor = (element, url) => {
        if (!/^(https?:|file:|\/|\.|[^:]*$)/i.test(url)) {
            return undefined;
        }
        const found = findAsset(url);
        // Frames from elsewhere depend on being there, they're usually widgets that talk to servers
        if (!found || new URL(found.url).origin !== new URL(PAGE_URL).origin) {
            return "about:blank";
        }
        element.sandbox = "";
        return "data:text/html;charset=utf-8," + encodeURIComponent(toText(found));
    };
    const replacementFor = (element, url) =>
        element instanceof HTMLIFrameElement ? frameUrlFor(element, url) : dataUrlFor(url);

    const attributes = [
        [HTMLIFrameElement, "src"],
        [HTMLImageElement, "src"],
        [HTMLLinkElement, "href"],
        [HTMLScriptElement, "src"],
        [HTMLSourceElement, "src"],
    ];
    for (const [element, attribute] of attributes) {
        const descriptor = Object.getOwnPropertyDescriptor(element.prototype, attribute);
        Object.defineProperty(element.prototype, attribute, {
            ...descriptor,
            set(value) {
                descriptor.set.call(this, replacementFor(this, String(value)) || value);
            },
        });
    }
    // Bundlers set integrity of what they load, but what's in here has already been through that,
    // and scripts have been modified since
    for (const element of [HTMLLinkElement, HTMLScriptElement]) {
        const descriptor = Object.getOwnPropertyDescriptor(element.prototype, "integrity");
        Object.defineProperty(element.prototype, "integrity", { ...descriptor, set() {} });
    }

    const originalSetAttribute = Element.prototype.setAttribute;
    Element.prototype.setAttribute = function (name, value) {
        if (String(name).toLowerCase() === "integrity") {
            return undefined;
        }
        const isTarget = attributes.some(
            ([element, attribute]) => this instanceof element && attribute === String(name).toLowerCase(),
        );
        return originalSetAttribute.call(this, name, (isTarget && replacementFor(this, String(value))) || value);
    };
    // Markup that comes from HTML strings doesn't go through any of that
    const fixElement = (element) => {
        if (element.nodeType !== 1) {
            return;
        }
        for (const node of [element, ...element.querySelectorAll("iframe[src],img[src],link[href],source[src]")]) {
            const name = node.matches("link") ? "href" : "src";
            const value = node.matches("iframe,img,link,source") && node.getAttribute(name);
            const url = value && replacementFor(node, value);
            if (url) {
                originalSetAttribute.call(node, name, url);
            }
        }
    };
    new MutationObserver((mutations) => {
        for (const mutation of mutations) {
            mutation.addedNodes.forEach(fixElement);
        }
    }).observe(document.documentElement, { childList: true, subtree: true });

    // Import maps don't apply to workers, and browsers limit how long URLs can be. Workers start
    // small, and then get sent everything else, which they turn into modules on their own side.

    const isScript = (response) => /javascript|ecmascript/.test(response.type);

    const collectModules = (entryUrl) => {
        const modules = [];
        const visited = new Set();
        const visit = (url) => {
            const found = responses.find((response) => response.url === url);
            if (!found || visited.has(url)) {
                return;
            }
            visited.add(url);
            const code = toText(found);
            for (const [, , target] of code.matchAll(/(\bfrom\s*|\bimport\s*)"(https?:[^"]+)"/g)) {
                visit(target);
            }
            modules.push([url, code]);
            for (const [, baseUrl, specifier] of code.matchAll(/__monolithImport\("([^"]+)",\s*"([^"]+)"/g)) {
                visit(new URL(specifier, baseUrl).href);
            }
        };
        visit(entryUrl);
        return modules;
    };

    const startWorker = async (event, installLocation, installNetwork, installClock, installImports) => {
        const { capturedAt, entryUrl, isModule, modules, responses } = event.data;
        const heldMessages = [];
        const hold = (message) => heldMessages.push(message);
        self.addEventListener("message", hold);
        installLocation(entryUrl);
        installNetwork(responses, entryUrl);
        installClock(capturedAt);

        const toUrl = (code) => URL.createObjectURL(new Blob([code], { type: "text/javascript" }));
        const moduleUrls = {};
        for (const [url, code] of modules) {
            moduleUrls[url] = toUrl(
                code.replace(/(\bfrom\s*|\bimport\s*)"(https?:[^"]+)"/g, (statement, keyword, target) =>
                    moduleUrls[target] ? keyword + JSON.stringify(moduleUrls[target]) : statement,
                ),
            );
        }
        installImports(moduleUrls);

        if (isModule) {
            await import(moduleUrls[entryUrl]);
        } else {
            importScripts(moduleUrls[entryUrl]);
        }
        self.removeEventListener("message", hold);
        for (const message of heldMessages) {
            self.dispatchEvent(new MessageEvent("message", { data: message.data, ports: [...message.ports] }));
        }
    };

    const OriginalWorker = window.Worker;
    if (OriginalWorker) {
        window.Worker = class extends OriginalWorker {
            constructor(url, options) {
                const found = findAsset(String(url));
                if (!found) {
                    super(url, options);
                    return;
                }
                const start =
                    'self.addEventListener("message",(event)=>{(' + startWorker + ")(event," +
                    installLocation + "," + installNetwork + "," + installClock + "," + installImports + ")" +
                    ".catch((error)=>setTimeout(()=>{throw error}))},{once:true});";
                super("data:text/javascript;charset=utf-8," + encodeURIComponent(start), options);
                OriginalWorker.prototype.postMessage.call(this, {
                    capturedAt: CAPTURED_AT,
                    entryUrl: found.url,
                    isModule: Boolean(options && options.type === "module"),
                    modules: collectModules(found.url),
                    responses: responses.filter((response) => !isScript(response)),
                });
            }
        };
    }

    // Scripts read the location from here, see rewrite_imports()

    const installLocation = (pageUrl) => {
        const realLocation = self.location;
        const entries = [{ state: null, url: pageUrl }];
        let index = 0;
        let current = new URL(pageUrl);

        const fireHistoryEvents = (previousUrl) => {
            self.dispatchEvent(new PopStateEvent("popstate", { state: entries[index].state }));
            if (previousUrl.hash !== current.hash) {
                self.dispatchEvent(new HashChangeEvent("hashchange", { newURL: current.href, oldURL: previousUrl.href }));
            }
        };

        const setEntry = (state, url, isNew) => {
            if (isNew) {
                entries.length = index + 1;
                entries.push({ state, url });
                index += 1;
            } else {
                entries[index] = { state, url };
            }
            current = new URL(url);
        };

        // Staying within the page is the only thing that can work, it's all that was saved
        const navigate = (url, isNew) => {
            const target = new URL(url, current);
            if (target.origin !== current.origin) {
                realLocation.assign(target.href);
                return;
            }
            const previousUrl = current;
            setEntry(null, target.href, isNew);
            setTimeout(() => fireHistoryEvents(previousUrl));
        };

        const fakeLocation = {
            assign: (url) => navigate(String(url), true),
            reload: () => realLocation.reload(),
            replace: (url) => navigate(String(url), false),
            toString: () => current.href,
        };
        for (const name of ["hash", "host", "hostname", "href", "origin", "password", "pathname", "port", "protocol", "search", "username"]) {
            Object.defineProperty(fakeLocation, name, {
                enumerable: true,
                get: () => current[name],
                set: (value) => {
                    if (name === "href") {
                        navigate(String(value), true);
                    } else {
                        const target = new URL(current);
                        target[name] = value;
                        navigate(target.href, true);
                    }
                },
            });
        }

        const isDocument = (object) =>
            object === self || object === globalThis || (typeof document !== "undefined" && object === document);
        Object.defineProperty(Object.prototype, "__monolithLocation", {
            configurable: true,
            enumerable: false,
            get() {
                return isDocument(this) ? fakeLocation : this.location;
            },
            set(value) {
                if (isDocument(this)) {
                    fakeLocation.href = value;
                } else {
                    this.location = value;
                }
            },
        });

        if (typeof document === "undefined") {
            return;
        }

        const changeState = (isNew) => (state, title, url) => {
            setEntry(state, url === undefined || url === null ? current.href : new URL(url, current).href, isNew);
        };
        history.pushState = changeState(true);
        history.replaceState = changeState(false);
        history.go = (delta = 0) => {
            if (delta === 0) {
                realLocation.reload();
            } else if (entries[index + delta]) {
                const previousUrl = current;
                index += delta;
                current = new URL(entries[index].url);
                setTimeout(() => fireHistoryEvents(previousUrl));
            }
        };
        history.back = () => history.go(-1);
        history.forward = () => history.go(1);
        Object.defineProperty(history, "state", { get: () => entries[index].state });
        Object.defineProperty(history, "length", { get: () => entries.length });
        for (const name of ["URL", "baseURI", "documentURI"]) {
            Object.defineProperty(document, name, { get: () => current.href });
        }

        // Links that were left for the browser to follow would've led to files that don't exist
        window.addEventListener("click", (event) => {
            const link = event.target.closest && event.target.closest("a[href]");
            if (!link || event.defaultPrevented || event.button || event.ctrlKey || event.metaKey || event.shiftKey) {
                return;
            }
            const href = link.getAttribute("href");
            if (href.startsWith("#")) {
                // Leaves scrolling to the browser, this is only about keeping track of it
                setEntry(null, new URL(href, current).href, true);
            } else if (!link.target && !link.hasAttribute("download") && new URL(href, current).origin === current.origin) {
                event.preventDefault();
                navigate(href, true);
            }
        });
    };
    installLocation(PAGE_URL);

    // Scripts call this instead of import(), see rewrite_imports()

    const installImports = (moduleUrls) => {
        self.__monolithImport = (baseUrl, specifier, options) => {
            const text = String(specifier);
            const url = /^\.{0,2}\//.test(text) ? new URL(text, baseUrl).href : text;
            return import(moduleUrls[url] || url, options);
        };
    };
    installImports({});

    // Browsers don't keep cookies for files, this keeps them for as long as the page stays open

    const cookies = new Map();
    Object.defineProperty(document, "cookie", {
        configurable: true,
        get: () => Array.from(cookies, ([name, value]) => name + "=" + value).join("; "),
        set: (cookie) => {
            const [pair, ...attributes] = String(cookie).split(";");
            const separator = pair.indexOf("=");
            const name = pair.slice(0, separator).trim();
            const value = pair.slice(separator + 1).trim();
            const attribute = (wanted) => {
                const found = attributes.find((item) => item.trim().toLowerCase().startsWith(wanted + "="));
                return found && found.slice(found.indexOf("=") + 1);
            };
            const maxAge = attribute("max-age");
            const expires = attribute("expires");
            const isExpired =
                (maxAge !== undefined && Number(maxAge) <= 0) ||
                (maxAge === undefined && expires !== undefined && new Date(expires) <= new Date());

            if (separator < 0 || isExpired) {
                cookies.delete(separator < 0 ? pair.trim() : name);
            } else {
                cookies.set(name, value);
            }
        },
    });

    // Time resumes from the moment the page was saved, cached data shouldn't look expired

    const installClock = (capturedAt) => {
        const originalDate = Date;
        const offset = capturedAt - originalDate.now();
        function SavedDate(...args) {
            if (!new.target) {
                return new originalDate(originalDate.now() + offset).toString();
            }
            const parameters = args.length ? args : [originalDate.now() + offset];
            return Reflect.construct(originalDate, parameters, new.target);
        }
        SavedDate.prototype = originalDate.prototype;
        SavedDate.prototype.constructor = SavedDate;
        SavedDate.now = () => originalDate.now() + offset;
        SavedDate.parse = originalDate.parse;
        SavedDate.UTC = originalDate.UTC;
        self.Date = SavedDate;
    };
    installClock(CAPTURED_AT);
})();
