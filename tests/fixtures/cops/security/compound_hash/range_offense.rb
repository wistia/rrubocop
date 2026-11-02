def hash; a.hash ^ b.hash; end
# rrubocop-expect: 1:10 Security/CompoundHash: Use `[...].hash` instead of combining hash values manually.
