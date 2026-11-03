"""Trusted UTC primitive conversions and preserved nil/foreign-zone/type contracts."""
def programs():
    require="gem 'activesupport','8.1.3.1'\nrequire 'active_support/all'\n"
    utc="ENV['TZ']='UTC'\nTime.zone='UTC'\n"
    result=[]
    config={'EnforcedStyle':'strict','__RubocopRailsVersion':'2.26.0'}
    for body in ('Time.now; puts :done','Time.new; puts :done','p Time.at(0).to_i','p Time.at(123.5).to_f','p Time.new(2020,1,2,3,4,5).to_i','p Time.local(2020,1,2).to_i','p Time.parse("2020-01-02 03:04:05").to_i','p "2020-01-02 03:04:05".to_time.to_i','p Time.new(2020).in_time_zone.to_i','p ::Time.new(2020).to_i'):
        result.append(('Rails/TimeZone',require+utc+body+'\n',config))
    for setup,body in (
        ("ENV['TZ']='UTC'\nTime.zone=nil\n",'p Time.new(2020).to_i'),
        ("ENV['TZ']='EST5'\nTime.zone='UTC'\n",'p Time.new(2020).to_i'),
        ("ENV['TZ']='UTC'\nTime.zone='Eastern Time (US & Canada)'\n",'p Time.new(2020).to_i'),
        (utc,'p Time.new(2020).class.name'),
        (utc,'p Time.at(0)'),
        (utc+"ENV.store('TZ','EST5')\n",'p Time.new(2020).to_i'),
        (utc+'value=1\n','p Time.at(0).to_i'),
        (utc,'p "2020-01-02".to_time.class.name'),
        (utc,'p "2020-01-02"&.to_time.to_i'),
    ):
        result.append(('Rails/TimeZone',require+setup+body+'\n',config))
    for body in ('p Time.new(2020). in_time_zone.to_i','p ::\nTime.new(2020).to_i\np __LINE__'):
        result.append(('Rails/TimeZone',require+utc+body+'\n',config))
    result.append(('Rails/TimeZone',require+utc+'Kernel.p(Time.at(0).to_i)\n',config))
    override="class << Time\ndef parse(*); at(0); end\nend\n"
    result.append(('Rails/TimeZone',require+override+utc+'p Time.parse("2020-01-02").to_i\n',config))
    return result
