respond_to do |format|
 format.html
 format.json
end
# rrubocop-expect: 1:0 Custom/RequireRespondToAny: Every `respond_to` block must include a `format.any`.
