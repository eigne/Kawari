import csv,json,re,html,pathlib
root=pathlib.Path('.')
with open('target/feast-whm/Action-6.08.csv',encoding='utf-8') as f:
 r=csv.reader(f);next(r);headers=next(r);next(r);rows=[dict(zip(headers,x)) for x in r]
p=root/'resources/feast-actions.json'; catalog=json.loads(p.read_text())
def clean(s):return html.unescape(re.sub('<[^>]+>',' ',s)).replace('\xa0',' ').strip()
for job,slug,shared in [(22,'dragoon',34),(21,'warrior',19)]:
 catalog=[a for a in catalog if a['job']!=job]
 page=(root/f'target/feast-more-jobs/{slug}.html').read_text(encoding='utf-8-sig')
 for block in re.findall(r'<tr id="pvp_action__.*?</tr>',page,re.S):
  name=clean(re.search(r'<strong>(.*?)</strong>',block,re.S)[1])
  row=next(x for x in rows if x['Name']==name and x['ClassJob']==str(job) and x['IsPvP']=='True')
  desc=re.search(r'<td class="content">(.*?)</td>',block,re.S)[1]
  desc=re.sub(r'\s+',' ',clean(desc)).split('※')[0].strip()
  key=name.lower().replace("'",'').replace(' ','_')
  catalog.append(dict(job=job,key=key,id=int(row['#']),icon=int(row['Icon']),name=name,cast=int(row['Cast<100ms>'])/10,recast=int(row['Recast<100ms>'])/10,group=int(row['CooldownGroup'])-1,secondary=int(row['AdditionalCooldownGroup'])-1,charges=max(1,int(row['MaxCharges'])),range=int(row['Range']),radius=int(row['EffectRange']),description=desc,category='Weaponskill' if row['ActionCategory']=='3' else 'Ability',mpCost=0))
 keys={'smite','feint','bloodbath','arms_length','fetter_ward','backstep','raw_destruction','bolt','kit'} if job==22 else {'low_blow','full_swing','weapon_throw','reprisal','rampart','retaliation','aegis_boon','bolt','kit'}
 catalog += [dict(a,job=job) for a in catalog[:] if a['job']==shared and a['key'] in keys]
p.write_text(json.dumps(catalog,indent=2)+'\n',encoding='utf-8')
print('Added DRG/WAR records:',sum(a['job'] in (21,22) for a in catalog))
