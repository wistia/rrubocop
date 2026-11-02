stream_from(<<~TOPIC, coder: JSON)
 room
TOPIC
# rrubocop-expect: 1:22 AnyCable/StreamFrom: Custom coders are not supported in AnyCable
