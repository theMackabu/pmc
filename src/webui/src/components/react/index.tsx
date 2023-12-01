import ky from 'ky';
import Rename from '@/components/react/rename';
import { useEffect, useState, Fragment } from 'react';
import { Menu, Transition } from '@headlessui/react';
import { EllipsisVerticalIcon } from '@heroicons/react/20/solid';

const Index = () => {
	const [items, setItems] = useState([]);

	const badge = {
		online: 'bg-emerald-400',
		stopped: 'bg-red-500',
		crashed: 'bg-amber-400',
	};

	const fetch = () => {
		ky.get('/list')
			.json()
			.then((res) => setItems(res));
	};

	const classNames = (...classes: Array<any>) => classes.filter(Boolean).join(' ');
	const isRunning = (status: string): bool => (status == 'stopped' ? false : status == 'crashed' ? false : true);
	const action = (id: number, name: string) => ky.post(`/process/${id}/action`, { json: { method: name } }).then(() => fetch());

	useEffect(() => fetch(), []);

	return (
		<ul role="list" className="grid grid-cols-1 gap-x-6 gap-y-8 lg:grid-cols-4 xl:gap-x-8">
			{items.map((item) => (
				<li key={item.id} className="rounded-lg border border-zinc-700/50 bg-zinc-900/10 hover:bg-zinc-900/40 hover:border-zinc-700">
					<div className="flex items-center gap-x-4 border-b border-zinc-800/80 bg-zinc-900/20 px-4 py-3">
						<span className="text-md font-bold text-zinc-200 truncate">
							{item.name}
							<div className="text-xs font-medium text-zinc-400">{isRunning(item.status) ? item.pid : 'none'}</div>
						</span>
						<span className="relative flex h-2 w-2 -mt-3.5 -ml-2">
							<span className={`${badge[item.status]} relative inline-flex rounded-full h-2 w-2`}></span>
						</span>
						<Menu as="div" className="relative ml-auto">
							<Menu.Button className="transition border focus:outline-none focus:ring-0 focus:ring-offset-0 z-50 shrink-0 border-zinc-700/50 bg-transparent hover:bg-zinc-800 p-2 text-sm font-semibold rounded-lg ml-3">
								<EllipsisVerticalIcon className="h-5 w-5 text-zinc-50" aria-hidden="true" />
							</Menu.Button>
							<Transition
								as={Fragment}
								enter="transition ease-out duration-100"
								enterFrom="transform opacity-0 scale-95"
								enterTo="transform opacity-100 scale-100"
								leave="transition ease-in duration-75"
								leaveFrom="transform opacity-100 scale-100"
								leaveTo="transform opacity-0 scale-95">
								<Menu.Items className="absolute right-0 z-10 mt-2 w-48 origin-top-right rounded-lg bg-zinc-900 border border-zinc-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none text-base divide-y divide-zinc-800/50">
									<div className="p-1.5">
										<Menu.Item>
											{({ active }) => (
												<a
													onClick={() => action(item.id, 'restart')}
													className={classNames(
														active ? 'bg-blue-700/10 text-blue-500' : 'text-zinc-200',
														'rounded-md block px-2 py-2 w-full text-left cursor-pointer'
													)}>
													Reload
												</a>
											)}
										</Menu.Item>
										<Menu.Item>
											{({ active }) => (
												<a
													onClick={() => action(item.id, 'stop')}
													className={classNames(
														active ? 'bg-yellow-400/10 text-amber-500' : 'text-zinc-200',
														'rounded-md block p-2 w-full text-left cursor-pointer'
													)}>
													Terminate
												</a>
											)}
										</Menu.Item>
									</div>
									<div className="p-1.5">
										<Menu.Item>{({ active }) => <Rename process={item.id} callback={fetch} old={item.name} />}</Menu.Item>
									</div>
									<div className="p-1.5">
										<Menu.Item>
											{({ active }) => (
												<a
													onClick={() => action(item.id, 'delete')}
													className={classNames(
														active ? 'bg-red-700/10 text-red-500' : 'text-red-400',
														'rounded-md block p-2 w-full text-left cursor-pointer'
													)}>
													Delete
												</a>
											)}
										</Menu.Item>
									</div>
								</Menu.Items>
							</Transition>
						</Menu>
					</div>
					<a href={`/view?id=${item.id}`}>
						<dl className="-my-3 divide-y divide-zinc-800/30 px-6 py-4 text-sm leading-6">
							<div className="flex justify-between gap-x-1 py-1">
								<dt className="text-zinc-700">restarts</dt>
								<dd className="text-zinc-500">{item.restarts}</dd>
							</div>
							<div className="flex justify-between gap-x-1 py-1">
								<dt className="text-zinc-700">cpu</dt>
								<dd className="text-zinc-500">{isRunning(item.status) ? item.cpu : 'offline'}</dd>
							</div>
							<div className="flex justify-between gap-x-1 py-1">
								<dt className="text-zinc-700">memory</dt>
								<dd className="text-zinc-500">{isRunning(item.status) ? item.mem : 'offline'}</dd>
							</div>
							<div className="flex justify-between gap-x-1 py-1">
								<dt className="text-zinc-700">uptime</dt>
								<dd className="text-zinc-500">{isRunning(item.status) ? item.uptime : 'none'}</dd>
							</div>
						</dl>
					</a>
				</li>
			))}
		</ul>
	);
};

export default Index;
