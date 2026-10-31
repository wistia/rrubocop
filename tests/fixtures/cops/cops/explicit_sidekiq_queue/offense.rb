class Work < BaseWorker; end
# rrubocop-expect: 1:0 Cops/ExplicitSidekiqQueue: Sidekiq workers must explicitly declare their queue. Use `sidekiq_options queue: "queue_name"` or `use_default_queue!` to opt into the default queue.
