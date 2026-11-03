"""Trusted non-AR receivers, result contracts and actual ActiveModel error slices."""
def programs():
    result=[]
    setup='$events=[]\nUser=Struct.new(:all).new([1,2,3])\n'
    for body in ('p(User.all.each { |v| v*2 })','result=User.all.each { |v| v*2 }; p result','User.all.each { |v| $events << v }; p $events','p User.all.each.class','User.all.each { |v| break :stopped }; puts :finished'):
        result.append(('Rails/FindEach',setup+body+'\n'))
    setup='$events=[]\nclass Relation\ndef each(&block); $events << :each; [1,2,3].each(&block); end\ndef find_each(&block); $events << :find_each; [1,2,3].each(&block); nil; end\nend\nclass User\ndef self.all; Relation.new; end\nend\n'
    for body in ('p(User.all.each { |v| v })','User.all.each { |v| $events << v }; p $events','p User.all.each.class'):
        result.append(('Rails/FindEach',setup+body+'\n'))
    result.append(('Rails/FindEach',"gem 'activemodel','8.1.3.1'\nrequire 'active_model'\nclass Probe\ninclude ActiveModel::Validations\nend\nmodel=Probe.new\nmodel.errors.add(:name,:blank)\nmodel.errors.where(:name).each { |error| p error.type }\n"))
    return result
