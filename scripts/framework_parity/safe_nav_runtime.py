"""Explicitly authored ActiveSupport try programs; never execute arbitrary fixtures."""
def programs():
    header = "gem 'activesupport', '8.1.3.1'\nrequire 'active_support/core_ext/object/try'\n"
    rows = []
    def add(source, **config):
        row = ("Rails/SafeNavigation", header+source+"\n")
        rows.append(row+(config,) if config else row)
    add("$events=[]\ndef effect; $events << :argument; 'x'; end\np nil.try!(:concat,effect())\np $events")
    add("$events=[]\nclass Key\ndef hash; $events << :hash; 1; end\nend\nkey=Key.new\np nil.try!(:foo,{key=>1})\np $events")
    add('$events=[]\nclass Text\ndef to_s; $events << :string; "x"; end\nend\ntext=Text.new\np nil.try!(:foo,"#{text}")\np $events')
    add("p nil.try!(:foo,1)")
    add("p nil.try(:missing)",ConvertTry=True)
    add("p Object.new.try(:missing)",ConvertTry=True)
    setter="class Box\nattr_reader :value\ndef value=(value)\n@value=value\n:custom_return\nend\nend\nbox=Box.new\n"
    add(setter+"p box.try!(:value=,1)\np box.value")
    add(setter+"box.try!(:value=,1)\np box.value")
    add(setter+"begin\nbox.try!(:value=)\nrescue ArgumentError => error\np error.class\nend")
    add("p 'abc'.try!(:'upcase')")
    add("class Custom\ndef try!(*); :override; end\nend\np Custom.new.try!(:upcase)")
    add("class PrivateProbe\nprivate\ndef hidden; :hidden; end\npublic\ndef run\nbegin\ntry!(:hidden)\nrescue NoMethodError\np :private\nend\nend\nend\nPrivateProbe.new.run")
    add("p 'abc'.try!(:upcase)")
    add("p 'abc'.try(:upcase)",ConvertTry=True)
    add("p [1,2].try!(:join,',')")
    add("p [1,2].try(:join,',')",ConvertTry=True)
    add("p [1,2].try!(:map) { |value| value*2 }")
    add("p [1,2].try!(:each_with_object,[]) { |value,result| result << value*2 }")
    add("$events=[]\ndef effect; $events << :block; proc { :block }; end\np nil.try!(:foo,&effect())\np $events")
    add("p false.try!(:to_s)")
    add("p -1.try!(:abs)")
    add("p +1.try!(:abs)")
    add("nil.try!(:foo,\n 1)\np __LINE__")
    add("def __LINE__; 123; end\nnil.try!(:foo,\n 1)\np self.__LINE__")
    return rows
