class A
 include ::Sidekiq::Worker
 sidekiq_options queue: :work
 def perform; work; end
end
