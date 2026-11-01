class A < BaseWorker
 sidekiq_options :"qu\x65ue" => :default
 def perform; work; end
end
