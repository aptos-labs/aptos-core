export class MapWithDefault<K, V> extends Map<K, V> {
  private readonly default: () => V;

  get(key: K) {
    if (!this.has(key)) {
      this.set(key, this.default());
    }
    return super.get(key);
  }

  constructor(defaultFunction: () => V) {
    super();
    this.default = defaultFunction;
  }
}
