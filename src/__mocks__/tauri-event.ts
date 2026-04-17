export function listen(_event: string, _handler: (...args: any[]) => void) {
  return Promise.resolve(() => {})
}

export function emit(_event: string, _payload?: unknown) {
  return Promise.resolve()
}
