"""Both original Team target and Base requires_gem gates must pass independently."""
def extend(add):
    for cop,source in (("Rails/ContentTag","tag(:br)"),("Rails/IndexWith","x.map { |el| [el, el.to_sym] }.to_h"),("Rails/Pick","User.pluck(:name).first")):
        for target in (4.2,5.0,5.1,6.0,8.1):
            for gem in (None,"4.2.11.3","5.0.0","5.1.0","6.0.0","8.1.3.1"):
                add(cop,source,TargetRailsVersion=target,GemVersions={"railties":gem} if gem else {})
