RSpec.describe Worker do
 it do
  expect(Worker).to receive(
 # expectation
 :perform_async)
 end
end
# rrubocop-expect: 3:20 Cops/NoSidekiqReceive: receive(:perform_async) called without safe ancestor method. This could have test mask passing incorrect arguments to a worker.
