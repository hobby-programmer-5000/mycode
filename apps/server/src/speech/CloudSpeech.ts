import {
  CloudSpeechError,
  type CloudSpeechKeys,
  type CloudSpeechModel,
  type CloudSpeechProvider,
  type SecretReference,
  type ServerSettingsError,
  type SpeechTranscriptionOptions,
} from "@mycode/contracts";
import * as Context from "effect/Context";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Redacted from "effect/Redacted";
import * as ServerSettings from "../serverSettings.ts";
import { type SpeechInvalidAudioError } from "./SpeechService.ts";

export class CloudSpeech extends Context.Service<
  CloudSpeech,
  {
    readonly configured: Effect.Effect<CloudSpeechKeys, ServerSettingsError>;
    /** OpenRouter's public transcription catalog; always empty in offline local-only mode. */
    readonly models: Effect.Effect<ReadonlyArray<CloudSpeechModel>, CloudSpeechError>;
    readonly setKey: (
      provider: CloudSpeechProvider,
      key: Redacted.Redacted<string> | SecretReference | null,
    ) => Effect.Effect<void, ServerSettingsError>;
    readonly transcribe: (
      provider: CloudSpeechProvider,
      pcm: Uint8Array,
      options: SpeechTranscriptionOptions,
      model?: string | undefined,
    ) => Effect.Effect<string, CloudSpeechError | SpeechInvalidAudioError | ServerSettingsError>;
  }
>()("mycode/speech/CloudSpeech") {}

export const layer = Layer.effect(
  CloudSpeech,
  Effect.gen(function* () {
    const settings = yield* ServerSettings.ServerSettingsService;
    const configured = Effect.succeed<CloudSpeechKeys>({
      openai: false,
      xai: false,
      openrouter: false,
    });
    // Offline local-only mode: never contact OpenAI, xAI, or OpenRouter speech endpoints.
    const models = Effect.succeed<ReadonlyArray<CloudSpeechModel>>([]);
    const setKey = Effect.fn("CloudSpeech.setKey")(function* (
      provider: CloudSpeechProvider,
      key: Redacted.Redacted<string> | SecretReference | null,
    ) {
      yield* settings.updateSettings({
        cloudSpeechCredentials: {
          [provider]: Redacted.isRedacted(key) ? Redacted.value(key) : (key ?? ""),
        },
      });
    });
    const transcribe = Effect.fn("CloudSpeech.transcribe")(
      (
        provider: CloudSpeechProvider,
        _pcm: Uint8Array,
        _options: SpeechTranscriptionOptions,
        _model?: string,
      ) => Effect.fail(new CloudSpeechError({ provider, reason: "unavailable" })),
    );
    return CloudSpeech.of({ configured, models, setKey, transcribe });
  }),
);
