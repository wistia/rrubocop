describe Work do
 allow(Worker).to receive(:perform_async)
end
# rrubocop-expect: 2:18 Cops/NoSidekiqReceive: receive(:perform_async) called without safe ancestor method. This could have test mask passing incorrect arguments to a worker.
