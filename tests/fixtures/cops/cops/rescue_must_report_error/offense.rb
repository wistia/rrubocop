begin
 work
rescue ::
^ Cops/RescueMustReportError: Rescue blocks must report the error using `Sentry.capture_exception`, `Sentry.capture_message`, or `Rails.error.report`
 StandardError
 recover
end
