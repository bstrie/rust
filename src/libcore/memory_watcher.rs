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

pub enum Msg {
        pub ReportAllocation(Task, libc::uintptr_t, *libc::c_char, *libc::c_char),
	pub ReportDeallocation(Task, *libc::c_char),
	StopMemoryWatcher()
}

type MemoryWatcherKey = (int, libc::uintptr_t, libc::uintptr_t);

pub fn global_memory_watcher_spawner(msg_po: comm::Port<Msg>)
{	
	let mut hm_index:linear::LinearMap<int, @mut linear::LinearMap<libc::uintptr_t, MemoryWatcherKey>> = linear::LinearMap();
	
	loop { 
		match msg_po.recv() { 
			ReportAllocation(t, s, c, a) => {
				println("In message condition");
				let Metrics_value:MemoryWatcherKey = (*(t), s, (c as libc::uintptr_t));
				let test1:int = (*t);
				let val1 = hm_index.find(&test1);
				match val1 {
					Some(T) => {
					let hm_task_LinearMap:@mut linear::LinearMap<libc::uintptr_t, MemoryWatcherKey> = T;
					hm_task_LinearMap.insert((a as libc::uintptr_t), Metrics_value);
					}
					None => {
						let hm_task:@mut linear::LinearMap<libc::uintptr_t, MemoryWatcherKey> = @mut linear::LinearMap();
					hm_task.insert((a as libc::uintptr_t), Metrics_value);
					hm_index.insert(*(t), hm_task);
					}
				}				
			}
			ReportDeallocation(t, a) => {
				println("In deallocation condition");
				let val1 = hm_index.find(&*(t));
				match val1 {
					Some(T) => {
						let hm_task_deallocate:@mut linear::LinearMap<libc::uintptr_t, MemoryWatcherKey> = T;
						let val2 = hm_task_deallocate.remove(&(a as libc::uintptr_t));
						if(val2 == true) {
							println("Value removed");
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
	

#[test]
fn test_simple() {
let comm_memory_watcher_chan = get_memory_watcher_Chan();

//comm_memory_watcher_chan.send(ReportAllocation(task::get_task()));
let tid_recieve = task::get_task();
let tid = *(tid_recieve);
println(#fmt("current task id %d",tid));
memory_watcher_start();
let box = @0;
memory_watcher_stop();
}
