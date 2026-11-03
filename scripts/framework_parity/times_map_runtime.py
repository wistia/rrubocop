"""Authored iteration/error/override programs exercise Array.new correction guards."""
def programs():
    bodies=[
        "p(4.times.map { |i| i.to_s })",
        "p(0.times.map { |i| raise 'unreachable' })",
        "begin\np(-1.times.map { |i| i })\nrescue ArgumentError=>error\np error.class\nend",
        "begin\np(4.2.times.map { |i| i })\nrescue StandardError=>error\np [error.class,error.respond_to?(:name) ? error.name : nil]\nend",
        "begin\np(nil.times.map { |i| i })\nrescue StandardError=>error\np [error.class,error.respond_to?(:name) ? error.name : nil]\nend",
        "begin\np('4'.times.map { |i| i })\nrescue StandardError=>error\np [error.class,error.respond_to?(:name) ? error.name : nil]\nend",
        "begin\np([1,2].times.map { |i| i })\nrescue StandardError=>error\np [error.class,error.respond_to?(:name) ? error.name : nil]\nend",
        "n=-1\nbegin\np(n.times.map { |i| i })\nrescue ArgumentError=>error\np error.class\nend",
        "n=4\np(n.times.map { |i| i.to_s })",
        "module Shadow\nArray=Class.new do\ndef self.new(*); :shadow; end\nend\ndef self.run\n4.times.map { |i| i }\nend\nend\np Shadow.run",
        "class Integer\ndef times; [7,8].each; end\nend\np(2.times.map { |i| i })",
        "class << Array\ndef new(*); :changed; end\nend\np(2.times.map { |i| i })",
        "def foo(i); i.to_s; end\np(4.times.map(&method(:foo)))",
        "p(4.times.map { _1.to_s })",
        "p(4.times.map { |i| break :stopped })",
        "p(4.times.collect { |i| next i*2 })",
        "$events=[]\np(3.times.map { |i| $events << i; i*2 })\np $events",
        "p(3.times.\nmap { |i| __LINE__ })",
    ]
    bodies.extend(["p(4.times.map(&nil).class)","block=nil\np(4.times.map(&block).class)"])
    bodies.append("begin\np(9223372036854775808.times.map { |i| break :done })\nrescue RangeError=>error\np error.class\nend")
    return [("Performance/TimesMap",body+"\n") for body in bodies]
