describe A do
 it "works" do
 expect(Worker).to receive(:"perform_\x61sync")
                   ^ Cops/NoSidekiqReceive: receive(:perform_async) called without safe ancestor method. This could have test mask passing incorrect arguments to a worker.
 end
end
