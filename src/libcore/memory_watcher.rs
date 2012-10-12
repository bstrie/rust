use task::{spawn,Task};
use io::println;
use pipes::{stream,Port,Chan};
use private::{chan_from_global_ptr, weaken_task};
use comm::{Port, Chan, select2, listen};
use task::TaskBuilder;
use either::{Left, Right};
use send_map::linear;

#[abi = "cdecl"]
extern mod rustrt {
    fn rust_global_memory_watcher_chan_ptr() -> *libc::uintptr_t;
}

struct AllocationInfo {
	task:Task, 
	size:libc::uintptr_t,
	td:*libc::c_char,
	address_allocation:*libc::c_char
}

struct MetricsValue {
	task_id:int,
	size:libc::uintptr_t,
	td:*libc::c_char
}

pub enum Msg {
        pub ReportAllocation(Task, libc::uintptr_t, *libc::c_char, *libc::c_char),
	pub ReportDeallocation(Task, *libc::c_char),
	StopMemoryWatcher(),
	pub PrintMetrics(),
	pub ProcessMetrics(fn~(MetricsValue)),
	pub ProcessMetricsOfTask(fn~(MetricsValue), int),
	pub TestAllocationAddress(int, *libc::c_char)
}

type MemoryWatcherKey = (int, libc::uintptr_t, libc::uintptr_t);

pub fn global_memory_watcher_spawner(msg_po: comm::Port<Msg>)
{	
	let mut hm_index:linear::LinearMap<int, @mut linear::LinearMap<libc::uintptr_t, MetricsValue>> = linear::LinearMap();
	
	loop {
		match msg_po.recv() { 
			ReportAllocation(t, s, c, a) => {
				println(#fmt("%d %x",*(t),(s as uint)));
				let Metrics_value = MetricsValue {task_id:*(t), size:s, td:c };
				let test1:int = (*t);
				let val1 = hm_index.find(&test1);
				match val1 {
					Some(T) => {
					let hm_task_LinearMap:@mut linear::LinearMap<libc::uintptr_t, MetricsValue> = T;
					hm_task_LinearMap.insert((a as libc::uintptr_t), Metrics_value);
					}
					None => {
						let hm_task:@mut linear::LinearMap<libc::uintptr_t, MetricsValue> = @mut linear::LinearMap();
					hm_task.insert((a as libc::uintptr_t), Metrics_value);
					hm_index.insert(*(t), hm_task);
					}
				}				
			}
			ReportDeallocation(t, a) => {
				//println("In deallocation condition");
				let val1 = hm_index.find(&*(t));
				match val1 {
					Some(T) => {
						let hm_task_deallocate:@mut linear::LinearMap<libc::uintptr_t, MetricsValue> = T;
						let val2 = hm_task_deallocate.remove(&(a as libc::uintptr_t));
						if(val2 == true) {
							//println("Value removed");
						}
						else {
							println("Value not removed");
						}

					}
					None => {
					}
				}		
			}
			StopMemoryWatcher() => {
				break;
			}
			PrintMetrics() => {
				for hm_index.each_value |value| {
					let hm_task_printvalues = copy **value;
					for hm_task_printvalues.each_value |map_value| {
						let mut temp2 = map_value;
						let temp1 = temp2.task_id;
						println(#fmt("task id %d",temp1));
						println(#fmt("size is %x",(temp2.size as uint)));	
					}
				}
			}
			ProcessMetrics(func_process) => {
				for hm_index.each_value |value| {
					let hm_task_printvalues = copy **value;
					for hm_task_printvalues.each_value |map_value| {
						let mut temp2 = map_value;
						func_process(*map_value);	
					}
				}
			}
			ProcessMetricsOfTask(func_process,task_id) => {
				let val1 = hm_index.find(&task_id);
				match val1 {
					Some(T) => {
						let mut hm_task_printvalues = copy *T;
						for hm_task_printvalues.each_value |map_value| {
							let mut temp2 = map_value;
							func_process(*map_value);	
						}
					}
					None => {
						println("Task value not present in metrics");
					}
				}		
			}
			TestAllocationAddress(task_id, allocation_address) => {
				let val1 = hm_index.find(&task_id);
				match val1 {
					Some(T) => {
						let mut hm_task_searchvalues = copy *T;
						let val2 = hm_task_searchvalues.find(&(allocation_address as libc::uintptr_t));
						match val2 {
							Some(T) => {
								println(#fmt("Allocation address values found %x",(allocation_address as uint)));
							}
							None => {
								println("Allocation address value not present in metrics");
							}
						}
					}
					None => {
						println("Task value not present in metrics");
					}
				}
			}						
		}
	}
	
	do spawn {
		println("Hello");
	}
}

pub fn get_memory_watcher_Chan() -> comm::Chan<Msg> {

	let global_memory_watcher_ptr = rustrt::rust_global_memory_watcher_chan_ptr();

	unsafe {
		chan_from_global_ptr(global_memory_watcher_ptr, || {
                	task::task().sched_mode(task::SingleThreaded).unlinked()
            	}, global_memory_watcher_spawner)
	}
}

fn memory_watcher_start() {

	let global_memory_watcher_ptr = rustrt::rust_global_memory_watcher_chan_ptr();

	unsafe {
		let global_channel = chan_from_global_ptr(global_memory_watcher_ptr, || {
                			task::task().sched_mode(task::SingleThreaded).unlinked()
            				}, global_memory_watcher_spawner);
	}
	println("Memory watcher started");
}

fn memory_watcher_stop() {
	let comm_memory_watcher_chan = get_memory_watcher_Chan();

	comm_memory_watcher_chan.send(StopMemoryWatcher);
	println("Memory watcher stopped");
}

fn print_metrics() {
	let comm_memory_watcher_chan = get_memory_watcher_Chan();

	comm_memory_watcher_chan.send(PrintMetrics);
}

fn print_test1(metrics_test1:MetricsValue)
{

	println(#fmt("Print test1 %d",metrics_test1.task_id));
	println(#fmt("Print test1 %x",(metrics_test1.size) as uint));
}

#[test]
fn test_simple() {
let comm_memory_watcher_chan = get_memory_watcher_Chan();
let tid_recieve = task::get_task();
let tid = *(tid_recieve);
println(#fmt("current task id %d",tid));
memory_watcher_start();
unsafe {
let box = @0;
let addr: *libc::c_char = cast::reinterpret_cast(&box);
//comm_memory_watcher_chan.send(TestAllocationAddress(tid, addr));
}
do spawn {
	let tid_recieve_secondtask = task::get_task();
	let tid_secondtask = *(tid_recieve_secondtask);
	println(#fmt("Second task id %d",tid_secondtask));
	let box1 = @0;
	print_metrics();
	comm_memory_watcher_chan.send(ProcessMetricsOfTask(print_test1, tid_secondtask));
}
//print_metrics();
//comm_memory_watcher_chan.send(ProcessMetricsOfTask(print_test1, 4));
memory_watcher_stop();
}
