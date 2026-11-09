// Expectations execute unchanged production controllers, including timer closures.
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, expect, test, vi } from 'vitest';
import type { EchosightApi, Task, TaskSaveData } from '../../src/shared/types';
import { createTaskStateController } from '../../src/renderer/features/tasks/taskStateController';
import { createBackgroundTaskController } from '../../src/renderer/features/tasks/backgroundTaskController';
import { createTaskWorkflowController } from '../../src/renderer/features/tasks/taskWorkflowController';
import { createTriggerController } from '../../src/renderer/features/tasks/triggerController';
import { showBackgroundTaskNotification } from '../../src/renderer/features/tasks/backgroundTaskUi';

const epoch = 1_800_000_000_000;
const iso = (n: number) => new Date(n).toISOString();
const clone = <T>(x: T): T => JSON.parse(JSON.stringify(x));
const task = (id: number, extra: Partial<Task> = {}): Task => ({ id, text: `Task ${id}`, completed: false,
  createdAt: iso(epoch), children: [], mode: 'main', triggers: [], activated: true,
  activatedAt: null, backgroundOptions: null, ...extra });
const bg = (id: number, minutes: number | null = 1, extra: Partial<Task> = {}): Task => task(id, {
  mode: 'background', activated: false, backgroundOptions: { expiresAfterMinutes: minutes, priority: 'normal' }, ...extra });
type Op = { op: string; [key: string]: any };
type Scenario = { name: string; tasks: Task[]; ops: Op[] };
const scenarios: Scenario[] = [
  { name: 'nested-parent-activation', tasks: [task(1, { triggers: [20], children: [task(2, { triggers: [21], children: [task(3, { triggers: [22,22,999,1] })] })] }), bg(20), bg(21), bg(22)], ops: [
    { op: 'next' }, { op: 'advance', ms: 10000 }, { op: 'toggle', id: 3 }, { op: 'undo' }, { op: 'advance', ms: 50000 }, { op: 'redo' }, { op: 'undo' }, { op: 'save' }] },
  { name: 'parent-toggle-not-child-triggers', tasks: [task(1, { triggers: [20], children: [task(2, { triggers: [21] })] }), bg(20), bg(21)], ops: [{ op: 'toggle', id: 1 }, { op: 'toggle', id: 2 }, { op: 'toggle', id: 2 }] },
  { name: 'active-completed-backgrounds', tasks: [task(1, { triggers: [20,21,22,23] }), bg(20,1,{ activated:true, activatedAt:iso(epoch-10000) }), bg(21,1,{completed:true}), bg(22,null), bg(23,0)], ops: [
    { op: 'restart' }, { op: 'toggle', id: 1 }, { op: 'toggle', id: 20 }, { op: 'advance', ms:60000 }, { op: 'toggle', id:1 }, { op: 'undo' }, { op: 'redo' }] },
  { name: 'expiry-tolerance', tasks: [bg(10,-1,{activated:true}), bg(11,0,{activated:true}), bg(12,0.001,{activated:true}), bg(13,1,{activated:true,activatedAt:'not a date'}), bg(14,1,{activated:true,activatedAt:iso(epoch-60000)}), bg(15,1,{activated:true,activatedAt:iso(epoch+5000)}), bg(16,null,{activated:true})], ops: [
    { op:'restart' }, { op:'advance',ms:59 }, { op:'advance',ms:1 }, { op:'advance',ms:59940 }, { op:'restart' }, { op:'advance',ms:5000 }] },
  { name: 'restart-cancel-reset', tasks: [bg(20,1,{activated:true}),bg(21,2,{activated:true}),bg(22,1,{activated:true,activatedAt:'invalid'})], ops: [
    { op:'restart' }, { op:'advance',ms:20000 }, { op:'restart' }, { op:'cancel',id:21 }, { op:'advance',ms:40000 }, { op:'cancelAll' }, { op:'advance',ms:40000 }, { op:'restart' }, { op:'advance',ms:120000 }] },
  { name: 'remove-and-history', tasks: [task(1,{triggers:[20,21]}), task(2,{children:[bg(20)]}), bg(21,2)], ops: [
    {op:'next'}, {op:'advance',ms:10000}, {op:'delete',id:2}, {op:'undo'}, {op:'advance',ms:50000}, {op:'redo'}, {op:'undo'}, {op:'advance',ms:60000}, {op:'clear'}, {op:'undo'}, {op:'save'}] },
  { name:'replacement-captured-closure',tasks:[bg(20,1,{activated:true}),bg(21,2,{activated:true})],ops:[
    {op:'restart'}, {op:'replace',tasks:[bg(20,3,{activated:true,text:'Replacement'})]}, {op:'advance',ms:60000}, {op:'restart'}, {op:'advance',ms:60000}, {op:'cancelAll'}, {op:'advance',ms:180000}] },
  { name:'collision-timers-and-targets',tasks:[task(1,{triggers:[20,20]}),bg(20),bg(20,2,{activated:true,text:'Second same id'}),task(2,{children:[bg(21,1,{activated:true}),bg(21,2,{activated:true,text:'Nested duplicate'})]})],ops:[
    {op:'restart'}, {op:'toggle',id:1}, {op:'advance',ms:60000}, {op:'advance',ms:60000}, {op:'undo'}, {op:'redo'}, {op:'delete',id:20}, {op:'restart'}, {op:'advance',ms:120000}] },
  { name:'colliding-complete-next',tasks:[task(1,{completed:true,triggers:[20]}),task(1,{triggers:[21]}),bg(20),bg(21)],ops:[{op:'next'},{op:'undo'},{op:'redo'},{op:'advance',ms:60000}] },
  { name:'trigger-edit-history-badges',tasks:[task(1,{triggers:[21,20]}),bg(20,1,{backgroundOptions:{expiresAfterMinutes:1,priority:'high'}}),task(2,{children:[bg(21)]})],ops:[
    {op:'configure',id:1}, {op:'editDormant',id:20,text:'Changed dormant',priority:'normal'}, {op:'triggers',id:1,selected:[20,21]}, {op:'undo'}, {op:'triggers',id:1,selected:[20]}, {op:'redo'}, {op:'triggers',id:1,selected:[20]}, {op:'undo'}, {op:'redo'}, {op:'toggle',id:1}, {op:'save'}] },
  { name:'direct-activation-immediate-expiry',tasks:[bg(20,-1),bg(21,1,{completed:true}),bg(22)],ops:[{op:'activate',triggers:[20,20,21,22,22,999]},{op:'activate',triggers:[21,22]},{op:'deactivate',triggers:[21,22]},{op:'advance',ms:60000},{op:'capture',label:'manual replacement'},{op:'replace',tasks:[]},{op:'undo'},{op:'redo'}] },
  { name:'nested-colliding-leaves',tasks:[task(1,{children:[task(3,{triggers:[20]})]}),task(2,{children:[task(3,{triggers:[21]})]}),bg(20),bg(21)],ops:[{op:'next'},{op:'next'},{op:'undo'},{op:'redo'},{op:'advance',ms:60000}] },
  { name:'legacy-dates-and-timer-limits',tasks:[bg(20,1,{activated:true,activatedAt:'2027-01-15'}),bg(21,1,{activated:true,activatedAt:'2027-01-15T09:00:00+01:00'}),bg(22,1e10,{activated:true}),bg(23,0.000001,{activated:true}),bg(1749938954072.5596,1,{activated:true}),bg(-1,1,{activated:true})],ops:[{op:'restart'},{op:'advance',ms:1},{op:'cancelAll'},{op:'restart'},{op:'advance',ms:60000}] },
  { name:'dormant-clear-and-empty-history',tasks:[bg(20)],ops:[{op:'clear'},{op:'undo'},{op:'redo'},{op:'delete',id:999},{op:'toggle',id:999},{op:'next'},{op:'triggers',id:999,selected:[]}] },
  { name:'create-tolerant-options',tasks:[task(1)],ops:[
    ...[null,{}, {expiresAfterMinutes:'2.9',priority:'high'}, {expiresAfterMinutes:true},{expiresAfterMinutes:0.5},{expiresAfterMinutes:-1},{expiresAfterMinutes:[]}].map((options,i)=>({op:'addBackground',text:`New ${i}`,options})), {op:'undo'},{op:'redo'},{op:'save'}] }
];
const sourcePaths = ['backgroundTaskController','triggerController','taskWorkflowController','taskStateController','backgroundTaskUi','taskCompletion','taskCreation','taskTree','taskMutations','taskPersistence','taskProgress','taskEditUi'].map(n=>`src/renderer/features/tasks/${n}.ts`);
const provenance = Object.fromEntries(sourcePaths.map(p=>[p,createHash('sha256').update(readFileSync(p)).digest('hex')]));
const out = resolve('crates/echosight-core/tests/golden/background');
function emit(name: string, value: unknown) {
  const path = resolve(out,name); const bytes = JSON.stringify(value,null,2);
  if (process.env.BACKGROUND_GOLDEN_CHECK === '1') expect(readFileSync(path,'utf8'),name).toBe(bytes);
  else { mkdirSync(out,{recursive:true}); writeFileSync(path,bytes); }
}
afterEach(()=>{vi.restoreAllMocks();vi.useRealTimers();document.body.replaceChildren();});
for (const s of scenarios) test(s.name, async()=>{
  vi.useFakeTimers(); vi.setSystemTime(epoch); vi.spyOn(Math,'random').mockReturnValue(0.017);
  vi.spyOn(window,'focus').mockImplementation(()=>{});
  document.body.innerHTML = `<div id="configureTriggersModal"><div id="triggerTaskList"></div><input id="newBgTaskInput"><input id="bgTaskHighPriority" type="checkbox"></div><div id="taskEditModal"><input id="taskEditInput"><div id="taskEditBackgroundOptions"><input id="taskEditHighPriority" type="checkbox"></div></div>`;
  let effects: any[] = [];
  const emitEffect = (value: unknown): void => { effects.push(clone(value)); };
  const api = { saveTasks: async(data:TaskSaveData)=>{emitEffect({kind:'persist',data});return {success:true};} } as EchosightApi;
  const state = createTaskStateController({api,logger:{log(){},error(){}}}); state.replaceTasks(clone(s.tasks),null);
  // Observe only actual background timers; modal focus/toast timers aren't domain timers.
  let timerScope = false; let schedulingId: number | null = null;
  const pending = new Map<any,{id:number,dueMs:number}>();
  const nativeSet = globalThis.setTimeout; const nativeClear = globalThis.clearTimeout;
  vi.spyOn(globalThis,'setTimeout').mockImplementation(((fn:any,delay:any,...args:any[])=>{
    if (!timerScope) return nativeSet(fn,delay,...args);
    const capturedId = schedulingId!;
    let handle:any; handle=nativeSet(()=>{const old=timerScope;timerScope=true;try{fn(...args);}finally{timerScope=old;pending.delete(handle);}},delay);
    const clock = (nativeSet as any).clock;
    const timer={id:capturedId,dueMs:clock.timers[Number(handle)].callAt};
    pending.set(handle,timer);emitEffect({kind:'schedule',...timer});return handle;
  }) as any);
  vi.spyOn(globalThis,'clearTimeout').mockImplementation(((handle:any)=>{
    const timer=pending.get(handle);if(timer){emitEffect({kind:'cancel',id:timer.id});pending.delete(handle);}nativeClear(handle);
  }) as any);
  // Timer ID is observed from the real closure's task argument via activation/restart
  // traversal, not inferred from duplicate IDs. A setTimeout stack has no argument
  // task; use the controller's log-free scheduling order and task activatedAt getter.
  const attach = () => {
    const visit=(tasks:Task[])=>tasks.forEach(t=>{
      let value=t.activatedAt;
      Object.defineProperty(t,'activatedAt',{enumerable:true,configurable:true,get(){if(timerScope)schedulingId=t.id;return value;},set(v){value=v;}});
      visit(t.children);
    });visit(state.getTasks());
  };
  attach();
  const changed=()=>{void state.saveTasks();};
  const raw=createBackgroundTaskController({getTasks:()=>state.getTasks(),onChanged:changed,onActivated:count=>{
    const wasTimerScope=timerScope;timerScope=false;
    try {
      showBackgroundTaskNotification(count);
      const notification=document.querySelector('.bg-task-notification')!;
      emitEffect({kind:'activated',count,message:notification.textContent});
      notification.remove(); // Visual lifetime is outside the domain timer oracle.
    } finally { timerScope=wasTimerScope; }
    emitEffect({kind:'sound',event:'backgroundActivated'});
  },logger:{log(message){emitEffect({kind:'log',message});}}});
  const background = new Proxy(raw,{get(target,key){const fn=(target as any)[key];return (...args:any[])=>{const old=timerScope;timerScope=true;try{return fn(...args);}finally{timerScope=old;}};}});
  const workflow=createTaskWorkflowController({backgroundTasks:background,taskState:state,confirmUser:async()=>true,isInteractive:()=>false,renderTasks(){},updateProgress(){},saveTasks:()=>state.saveTasks(),playThemeSound:event=>emitEffect({kind:'sound',event}),logger:{log(){},error(e){throw e;}}});
  // History replaces object identities; attach observation before the real restart.
  const restart=raw.restartExpirationTimers;raw.restartExpirationTimers=()=>{attach();restart();};
  const triggers=createTriggerController({getTasks:state.getTasks,findTaskById:state.findTaskById,captureUndoState:state.captureUndoState,addBackgroundTask:state.addBackgroundTask,onChanged:changed,openTaskEditor:workflow.openTaskEditor});
  let history: unknown = null;
  for (const name of ['undoLastAction','redoLastAction'] as const) {
    const original = state[name];
    vi.spyOn(state,name).mockImplementation(()=>{const result=original();history=clone(result);return result;});
  }
  const snapshots:any[]=[];
  for(const op of s.ops){
    effects=[];history=null;let metadata:any=null;
    switch(op.op){
      case 'activate':background.activateTriggeredTasks(task(0,{triggers:op.triggers}));break;
      case 'deactivate':background.deactivateTriggeredTasks(task(0,{triggers:op.triggers}));break;
      case 'capture':state.captureUndoState(op.label);break;
      case 'next':workflow.completeNextTask();break;
      case 'toggle':workflow.toggleTask(op.id);break;
      case 'restart':background.restartExpirationTimers();break;
      case 'cancel':background.clearExpirationTimer(op.id);break;
      case 'cancelAll':background.clearAllExpirationTimers();break;
      case 'advance':vi.advanceTimersByTime(op.ms);break;
      case 'undo':workflow.undoLastAction();break;
      case 'redo':workflow.redoLastAction();break;
      case 'delete':await workflow.deleteTask(op.id);break;
      case 'clear':await workflow.clearAllTasks();break;
      case 'replace':state.replaceTasks(clone(op.tasks),null);attach();break;
      case 'save':await state.saveTasks();break;
      case 'addBackground':state.addBackgroundTask(op.text,op.options);attach();break;
      case 'configure':
        triggers.configureTriggers(op.id);
        metadata=Array.from(document.querySelectorAll('.trigger-task-row')).map(row=>({id:Number((row.querySelector('input') as HTMLInputElement).dataset.bgTaskId),text:row.querySelector('.trigger-task-text')!.textContent,checked:(row.querySelector('input') as HTMLInputElement).checked,priorityBadge:row.querySelector('.trigger-badge-high')?.textContent??null,statusBadge:row.querySelector('.trigger-badge-active,.trigger-badge-dormant')!.textContent,editable:!!row.querySelector('button')}));break;
      case 'editDormant':
        triggers.configureTriggers(1);
        Array.from(document.querySelectorAll('.trigger-task-row')).find(row=>Number((row.querySelector('input') as HTMLInputElement).dataset.bgTaskId)===op.id)?.querySelector('button')?.click();
        (document.getElementById('taskEditInput') as HTMLInputElement).value=op.text;
        (document.getElementById('taskEditHighPriority') as HTMLInputElement).checked=op.priority==='high';workflow.saveTaskEdit();break;
      case 'triggers':
        triggers.configureTriggers(op.id);
        document.querySelectorAll<HTMLInputElement>('.trigger-checkbox').forEach(el=>el.checked=op.selected.includes(Number(el.dataset.bgTaskId)));
        triggers.saveTriggers();break;
      default:throw Error(op.op);
    }
    snapshots.push(clone({nowMs:Date.now(),tasks:state.getTasks(),timers:[...pending.values()].sort((a,b)=>a.id-b.id),effects,metadata,history}));
  }
  emit(`${s.name}.input.json`,{epoch,tasks:s.tasks,ops:s.ops});
  emit(`${s.name}.expected.json`,{provenance,snapshots});
});
