// Stub for `_generated/api` — any property access returns another Proxy
// so that `api.clients.list`, `internal.foo.bar`, etc. all resolve without error.

const handler: ProxyHandler<object> = {
  get: (_target, _prop) => new Proxy({}, handler),
};

export const api = new Proxy({}, handler);
export const internal = new Proxy({}, handler);
