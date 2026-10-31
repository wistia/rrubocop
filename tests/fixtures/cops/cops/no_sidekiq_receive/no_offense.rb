describe Work do
 allow(Worker).to receive(:perform_async).and_call_original
end
