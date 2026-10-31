begin; work; rescue; cleanup; end
# rrubocop-expect: 1:13 Cops/RescueMustReportError: Rescue blocks must report the error using `Sentry.capture_exception`, `Sentry.capture_message`, or `Rails.error.report`
