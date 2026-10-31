class WidgetPolicy; def show?; true; end; end
# rrubocop-expect: 1:20 Cops/EnforceNoShowInPolicies: Do not define a `show?` method in policies, `show?` logic should be encapsulated
