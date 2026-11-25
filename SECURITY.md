# Security (MyCode Experimental Build)

See [`.github/SECURITY.md`](.github/SECURITY.md).

This fork intentionally disables telemetry, cloud authentication, local/remote login gates, and connection limits for private experimental use. Do not expose this server directly to untrusted public networks, as every connected client receives full administrative scopes (`terminal:operate`, `orchestration:operate`, `filesystem:write`, etc.).
