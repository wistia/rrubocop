media.update_column(:"prot\x65cted", true)
^ Cops/NoCallbackSkippingProtectedWrites: Don't write `protected` with `update_column`; it skips the callback that syncs media files' S3 `private` tag. Use `update`/`save`.
