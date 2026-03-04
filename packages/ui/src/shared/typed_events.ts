export type EventMap = object;

export type EventListener<M extends EventMap> = <K extends keyof M>(
  type: K,
  payload: M[K],
) => void;

export class TypedEventHub<M extends EventMap> {
  private readonly listeners: Set<EventListener<M>> = new Set();

  on(listener: EventListener<M>): void {
    this.listeners.add(listener);
  }

  off(listener: EventListener<M>): void {
    this.listeners.delete(listener);
  }

  emit<K extends keyof M>(type: K, payload: M[K]): void {
    for (const listener of this.listeners) {
      listener(type, payload);
    }
  }
}
