import * as ClientTracer from "./clientTracer";

/** Client browser fingerprinting and remote OTLP trace exporting are permanently disabled. */
export function configureClientTracing(): Promise<void> {
  ClientTracer.setDelegate(null);
  return Promise.resolve();
}
