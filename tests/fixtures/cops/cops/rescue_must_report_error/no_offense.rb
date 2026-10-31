begin
 work
rescue
 Rails.error().report(error)
end
