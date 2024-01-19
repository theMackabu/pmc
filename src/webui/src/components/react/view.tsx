import { api } from '@/api';
import { matchSorter } from 'match-sorter';
import Rename from '@/components/react/rename';
import { Menu, Transition } from '@headlessui/react';
import { useEffect, useState, useRef, Fragment } from 'react';
import { EllipsisVerticalIcon } from '@heroicons/react/20/solid';

const classNames = (...classes: Array<any>) => classes.filter(Boolean).join(' ');

const formatMemory = (bytes: number): [number, string] => {
	const units = ['b', 'kb', 'mb', 'gb'];
	let size = bytes;
	let unitIndex = 0;

	while (size > 1024 && unitIndex < units.length - 1) {
		size /= 1024;
		unitIndex++;
	}

	return [+size.toFixed(1), units[unitIndex]];
};

const startDuration = (input: string): [number, string] => {
	const matches = input.match(/(\d+)([dhms])/);

	if (matches) {
		const value = parseInt(matches[1], 10);
		const unit = matches[2];

		return [value, unit];
	}

	return null;
};

const LogRow = ({ match, children }: any) => {
	const _match = match.toLowerCase();
	const chunks = match.length ? children.split(new RegExp('(' + match + ')', 'ig')) : [children];

	return (
		<div>
			{chunks.map((chunk: any, index: number) =>
				chunk.toLowerCase() === _match ? (
					<span key={index} className="bg-yellow-400 text-black">
						{chunk}
					</span>
				) : (
					<span key={index} className=" text-zinc-200">
						{chunk}
					</span>
				)
			)}
		</div>
	);
};

const LogViewer = (props: { server: string | null; base: string; id: number }) => {
	const [logs, setLogs] = useState<string[]>([]);
	const [loaded, setLoaded] = useState(false);
	const lastRow = useRef<HTMLDivElement | null>(null);
	const [searchQuery, setSearchQuery] = useState('');
	const [searchOpen, setSearchOpen] = useState(false);
	const [componentHeight, setComponentHeight] = useState(0);
	const filtered = (!searchQuery && logs) || matchSorter(logs, searchQuery);

	useEffect(() => {
		const updateComponentHeight = () => {
			const windowHeight = window.innerHeight;
			const newHeight = (windowHeight * 4) / 6;
			setComponentHeight(newHeight);
		};

		updateComponentHeight();
		window.addEventListener('resize', updateComponentHeight);

		return () => {
			window.removeEventListener('resize', updateComponentHeight);
		};
	}, []);

	const componentStyle = {
		height: componentHeight + 'px',
	};

	useEffect(() => {
		const handleKeydown = (event: any) => {
			if ((event.ctrlKey || event.metaKey) && event.key === 'f') {
				setSearchOpen(true);
				event.preventDefault();
			}
		};

		const handleKeyup = (event: any) => {
			if (event.key === 'Escape') {
				setSearchQuery('');
				setSearchOpen(false);
			}
		};

		const handleClick = () => {
			setSearchQuery('');
			setSearchOpen(false);
		};

		window.addEventListener('click', handleClick);
		window.addEventListener('keydown', handleKeydown);
		window.addEventListener('keyup', handleKeyup);

		return () => {
			window.removeEventListener('click', handleClick);
			window.removeEventListener('keydown', handleKeydown);
			window.removeEventListener('keyup', handleKeyup);
		};
	}, [searchOpen]);

	const loadLogs = () => {
		api
			.get(`${props.base}/process/${props.id}/logs/out`)
			.json()
			.then((data) => setLogs(data.logs))
			.finally(() => setLoaded(true));
	};

	const loadLogsRemote = () => {
		api
			.get(`${props.base}/remote/${props.server}/logs/${props.id}/out`)
			.json()
			.then((data) => setLogs(data.logs))
			.finally(() => setLoaded(true));
	};

	useEffect(() => (props.server != null ? loadLogsRemote() : loadLogs()), []);
	useEffect(() => lastRow.current?.scrollIntoView(), [loaded]);

	if (!loaded) {
		return <div className="text-lg text-white font-bold">loading...</div>;
	} else {
		return (
			<div>
				{searchOpen && (
					<div className="z-50 fixed top-[16.5rem] right-5 w-96 flex bg-zinc-800/50 backdrop-blur-md px-3 py-1 rounded-lg border border-zinc-700 shadow">
						<input
							className="grow bg-transparent p-2 border-0 text-white focus:ring-0 sm:text-sm"
							autoFocus
							placeholder="Filter logs..."
							value={searchQuery}
							onChange={(e) => setSearchQuery(e.target.value)}
						/>
						<span className="grow-0 text-zinc-400 font-medium mt-1.5">{searchQuery && filtered.length + ' matches'}</span>
					</div>
				)}
				<div className="p-5 pb-0 break-words overflow-y-scroll font-mono" style={componentStyle}>
					{filtered.map((log, index) => (
						<LogRow key={index} match={searchQuery}>
							{log}
						</LogRow>
					))}
					<div ref={lastRow} />
				</div>
			</div>
		);
	}
};

const View = (props: { id: string; base: string }) => {
	const [item, setItem] = useState<any>();
	const [loaded, setLoaded] = useState(false);
	const server = new URLSearchParams(window.location.search).get('server');

	const badge = {
		online: 'bg-emerald-400/10 text-emerald-400',
		stopped: 'bg-red-500/10 text-red-500',
		crashed: 'bg-amber-400/10 text-amber-400',
	};

	const fetch = () => {
		api
			.get(`${props.base}/process/${props.id}/info`)
			.json()
			.then((res) => setItem(res))
			.finally(() => setLoaded(true));
	};

	const fetchRemote = () => {
		api
			.get(`${props.base}/remote/${server}/info/${props.id}`)
			.json()
			.then((res) => setItem(res))
			.finally(() => setLoaded(true));
	};

	const isRunning = (status: string): bool => (status == 'stopped' ? false : status == 'crashed' ? false : true);
	const action = (id: number, name: string) => api.post(`${props.base}/process/${id}/action`, { json: { method: name } }).then(() => fetch());

	useEffect(() => (server != null ? fetchRemote() : fetch()), []);

	if (!loaded) {
		return <div className="text-lg text-white font-bold">loading...</div>;
	} else {
		const online = isRunning(item.info.status);
		const [uptime, upunit] = startDuration(item.info.uptime);
		const [memory, memunit] = formatMemory(online ? item.stats.memory_usage.rss : 0);

		const stats = [
			{ name: 'Status', value: item.info.status },
			{ name: 'Uptime', value: online ? uptime : 'none', unit: online ? upunit : '' },
			{ name: 'Memory', value: online ? memory : 'offline', unit: online ? memunit : '' },
			{ name: 'CPU', value: online ? item.stats.cpu_percent : 'offline', unit: online ? '%' : '' },
		];

		return (
			<Fragment>
				<div className="flex flex-col items-start justify-between gap-x-8 gap-y-4 bg-zinc-700/10 px-4 py-4 sm:flex-row sm:items-center sm:px-6 lg:px-8">
					<div>
						<div className="flex items-center gap-x-3">
							<h1 className="flex gap-x-1 text-base leading-7">
								<span className="font-semibold text-white cursor-default">{server != null ? `${server}/${item.info.name}` : item.info.name}</span>
							</h1>
							<div className={`flex-none rounded-full p-1 ${badge[item.info.status]}`}>
								<div className="h-2 w-2 rounded-full bg-current" />
							</div>
							{online && (
								<div className="order-first flex-none rounded-full bg-sky-400/10 px-2 py-0.5 text-xs font-medium text-sky-400 ring-1 ring-inset ring-sky-400/30 sm:order-none">
									{item.info.pid}
								</div>
							)}
						</div>
						<p className="text-xs leading-6 text-zinc-400">{item.info.command}</p>
					</div>
					<div className="mt-5 flex lg:ml-4 lg:mt-0">
						<span>
							<button
								type="button"
								onClick={() => action(props.id, 'restart')}
								className="disabled:opacity-50 transition inline-flex items-center justify-center space-x-1.5 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 saturate-[110%] border-zinc-700 hover:border-zinc-600 bg-zinc-800 text-zinc-50 hover:bg-zinc-700 px-4 py-2 text-sm font-semibold rounded-lg">
								{online ? 'Restart' : 'Start'}
							</button>
						</span>
						<span className="ml-3">
							<Menu as="div" className="relative inline-block text-left">
								<div>
									<Menu.Button className="transition inline-flex items-center justify-center space-x-1.5 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 border-zinc-700 bg-transparent hover:bg-zinc-800 p-2 text-sm font-semibold rounded-lg">
										<EllipsisVerticalIcon className="h-5 w-5 text-zinc-50" aria-hidden="true" />
									</Menu.Button>
								</div>

								<Transition
									as={Fragment}
									enter="transition ease-out duration-100"
									enterFrom="transform opacity-0 scale-95"
									enterTo="transform opacity-100 scale-100"
									leave="transition ease-in duration-75"
									leaveFrom="transform opacity-100 scale-100"
									leaveTo="transform opacity-0 scale-95">
									<Menu.Items className="absolute right-0 z-10 mt-2 w-48 origin-top-right rounded-lg bg-zinc-900/80 backdrop-blur-md border border-zinc-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none text-base divide-y divide-zinc-800/50">
										<div className="p-1.5">
											<Menu.Item>
												{({ active }) => (
													<a
														onClick={() => action(props.id, 'stop')}
														className={classNames(
															active ? 'bg-yellow-400/10 text-amber-500' : 'text-zinc-200',
															'rounded-md block p-2 w-full text-left cursor-pointer'
														)}>
														Terminate
													</a>
												)}
											</Menu.Item>
											<Menu.Item>
												{({ active }) => <Rename base={props.base} process={props.id} active={active} callback={fetch} old={item.info.name} />}
											</Menu.Item>
										</div>
										<div className="p-1.5">
											<Menu.Item>
												{({ active }) => (
													<a
														onClick={() => action(props.id, 'delete')}
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
						</span>
					</div>
				</div>

				<div className="grid grid-cols-1 bg-zinc-700/10 sm:grid-cols-2 lg:grid-cols-4">
					{stats.map((stat: any, index: number) => (
						<div
							key={stat.name}
							className={classNames(
								index % 2 === 1 ? 'sm:border-l' : index === 2 ? 'lg:border-l' : '',
								'border-t border-white/5 py-6 px-4 sm:px-6 lg:px-8'
							)}>
							<p className="text-sm font-medium leading-6 text-zinc-400">{stat.name}</p>
							<p className="mt-2 flex items-baseline gap-x-2">
								<span className="text-4xl font-semibold tracking-tight text-white">{stat.value}</span>
								{stat.unit ? <span className="text-sm text-zinc-400">{stat.unit}</span> : null}
							</p>
						</div>
					))}
				</div>

				<LogViewer server={server} id={parseInt(props.id)} base={props.base} />
			</Fragment>
		);
	}
};

export default View;
