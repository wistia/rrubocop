"""Production path and directive controls for the ten reviewed Rails cops."""
ROWS=[
 ('Rails/ActiveRecordAliases',"record.update_attributes(name: 'Ada')",'app/models/review_probe.rb','app/services/review_probe.rb'),
 ('Rails/ActiveSupportAliases',"'abc'.starts_with?('a')",'app/models/review_probe.rb','app/services/review_probe.rb'),
 ('Rails/ApplicationController','class Example < ActionController::Base; end','app/controllers/review_probe_controller.rb','app/models/review_probe.rb'),
 ('Rails/ApplicationJob','class Example < ActiveJob::Base; end','app/jobs/review_probe_job.rb','app/models/review_probe.rb'),
 ('Rails/ApplicationMailer','class Example < ActionMailer::Base; end','app/mailers/review_probe_mailer.rb','app/models/review_probe.rb'),
 ('Rails/ApplicationRecord','class Example < ActiveRecord::Base; end','app/models/review_probe.rb','db/review_probe.rb'),
 ('Rails/BelongsTo','belongs_to :blog, required: false','app/models/review_probe.rb','app/services/review_probe.rb'),
 ('Rails/HelperInstanceVariable','def helper; @value; end','app/helpers/review_probe_helper.rb','app/models/review_probe.rb'),
 ('Rails/RakeEnvironment','task :probe do; work; end','lib/tasks/review_probe.rake','lib/capistrano/tasks/review_probe.rake'),
 ('Rails/ReadWriteAttribute','read_attribute(:name)','app/models/review_probe.rb','app/services/review_probe.rb'),
]
def extra_cases():
    return [dict(cop=cop,source=source+' # rubocop:disable '+cop+'\n',path=path,positive=False) for cop,source,path,_ in ROWS] + [dict(cop='Rails/RakeEnvironment',source='task :probe do; work; end\n',path='Rakefile',positive=True)]
