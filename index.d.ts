/* auto-generated by NAPI-RS */
/* eslint-disable */
export declare class BindingBundleEndEventData {
  output: string
  duration: number
}

export declare class BindingError {
  kind: string
  message: string
}

export declare class BindingWatcher {
  constructor()
  start(listener: (data: BindingWatcherEvent) => void): Promise<void>
  loopSpawn(listener: (data: BindingWatcherEvent) => void): Promise<void>
}

export declare class BindingWatcherChangeData {
  path: string
  kind: string
}

export declare class BindingWatcherEvent {
  eventKind(): string
  watchChangeData(): BindingWatcherChangeData
  bundleEndData(): BindingBundleEndEventData
  bundleEventKind(): string
  errors(): Array<Error | BindingError>
}
