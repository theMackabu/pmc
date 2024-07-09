import { SSE, api, headers } from '@/api';
import { Switch } from '@headlessui/react';
import { matchSorter } from 'match-sorter';
import Loader from '@/components/react/loader';
import Rename from '@/components/react/rename';
import { useEffect, useState, useRef, Fragment } from 'react';
import { classNames, isRunning, formatMemory, startDuration } from '@/helpers';
import { EllipsisVerticalIcon, CheckIcon, ChevronUpDownIcon } from '@heroicons/react/20/solid';
import { Menu, MenuItem, MenuItems, MenuButton, Transition, Listbox, ListboxButton, ListboxOption, ListboxOptions } from '@headlessui/react';

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

const LogViewer = (props: { liveReload; setLiveReload; server: string | null; base: string; id: number }) => {
	const logTypes = [
		{ id: 1, name: 'stdout' },
		{ id: 2, name: 'stderr' }
	];

	const [logs, setLogs] = useState<string[]>([]);
	const [loaded, setLoaded] = useState(false);
	const [logType, setLogType] = useState(logTypes[0]);
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
		height: componentHeight + 'px'
	};

	useEffect(() => {
		const handleKeydown = (event: any) => {
			if ((event.ctrlKey || event.metaKey) && event.key === 'f') {
				setSearchOpen(true);
				props.setLiveReload(false);
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

	const fetchLogs = () => {
		const url =
			props.server != 'local'
				? `${props.base}/remote/${props.server}/logs/${props.id}/${logType.name}`
				: `${props.base}/process/${props.id}/logs/${logType.name}`;

		api
			.get(url)
			.json()
			.then((data) => setLogs(data.logs))
			.finally(() => setLoaded(true));
	};

	useEffect(() => {
		setLoaded(false);
		fetchLogs();
	}, [logType]);

	useEffect(() => {
		const fetchTime = setInterval(() => {
			if (props.liveReload) {
				fetchLogs();
				lastRow.current?.scrollIntoView();
			}
		}, 5000);

		return () => clearInterval(fetchTime);
	}, [props.liveReload]);

	useEffect(() => {
		lastRow.current?.scrollIntoView();
	}, [loaded]);

	if (!loaded) {
		return <Loader />;
	} else {
		return (
			<div>
				{searchOpen && (
					<div className="z-50 fixed top-[16.5rem] right-5 w-96 flex bg-zinc-800/50 backdrop-blur-md px-3 py-1 rounded-lg border border-zinc-700 shadow">
						<input
							className="grow bg-transparent p-2 border-0 text-white focus:ring-0 sm:text-sm placeholder-zinc-accent-fuchsia-500"
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
				<div className="absolute bottom-3 right-[125px]">
					<Switch
						checked={props.liveReload}
						onChange={props.setLiveReload}
						className="group relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent bg-zinc-800 transition-colors duration-200 ease-in-out focus:outline-none data-[checked]:bg-sky-500">
						<span className="sr-only">Set live reload</span>
						<span className="pointer-events-none relative inline-block h-5 w-5 transform rounded-full bg-zinc-500 group-data-[checked]:bg-white shadow ring-0 transition duration-200 ease-in-out group-data-[checked]:translate-x-5">
							<span
								aria-hidden="true"
								className="absolute inset-0 flex h-full w-full items-center justify-center transition-opacity duration-200 ease-in group-data-[checked]:opacity-0 group-data-[checked]:duration-100 group-data-[checked]:ease-out">
								<svg viewBox="0 0 16 16" fill="currentColor" className="h-3 w-3 text-zinc-100">
									<path
										fill-rule="evenodd"
										d="M4 2a1.5 1.5 0 0 0-1.5 1.5v9A1.5 1.5 0 0 0 4 14h8a1.5 1.5 0 0 0 1.5-1.5V6.621a1.5 1.5 0 0 0-.44-1.06L9.94 2.439A1.5 1.5 0 0 0 8.878 2H4Zm1 5.75A.75.75 0 0 1 5.75 7h4.5a.75.75 0 0 1 0 1.5h-4.5A.75.75 0 0 1 5 7.75Zm0 3a.75.75 0 0 1 .75-.75h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1-.75-.75Z"
										clip-rule="evenodd"
									/>
								</svg>
							</span>
							<span
								aria-hidden="true"
								className="absolute inset-0 flex h-full w-full items-center justify-center opacity-0 transition-opacity duration-100 ease-out group-data-[checked]:opacity-100 group-data-[checked]:duration-200 group-data-[checked]:ease-in">
								<svg viewBox="0 0 16 16" fill="currentColor" className="h-3 w-3 text-sky-500">
									<path
										fill-rule="evenodd"
										d="M5 4a.75.75 0 0 1 .738.616l.252 1.388A1.25 1.25 0 0 0 6.996 7.01l1.388.252a.75.75 0 0 1 0 1.476l-1.388.252A1.25 1.25 0 0 0 5.99 9.996l-.252 1.388a.75.75 0 0 1-1.476 0L4.01 9.996A1.25 1.25 0 0 0 3.004 8.99l-1.388-.252a.75.75 0 0 1 0-1.476l1.388-.252A1.25 1.25 0 0 0 4.01 6.004l.252-1.388A.75.75 0 0 1 5 4ZM12 1a.75.75 0 0 1 .721.544l.195.682c.118.415.443.74.858.858l.682.195a.75.75 0 0 1 0 1.442l-.682.195a1.25 1.25 0 0 0-.858.858l-.195.682a.75.75 0 0 1-1.442 0l-.195-.682a1.25 1.25 0 0 0-.858-.858l-.682-.195a.75.75 0 0 1 0-1.442l.682-.195a1.25 1.25 0 0 0 .858-.858l.195-.682A.75.75 0 0 1 12 1ZM10 11a.75.75 0 0 1 .728.568.968.968 0 0 0 .704.704.75.75 0 0 1 0 1.456.968.968 0 0 0-.704.704.75.75 0 0 1-1.456 0 .968.968 0 0 0-.704-.704.75.75 0 0 1 0-1.456.968.968 0 0 0 .704-.704A.75.75 0 0 1 10 11Z"
										clip-rule="evenodd"
									/>
								</svg>
							</span>
						</span>
					</Switch>
				</div>
				<Listbox className="absolute bottom-3 right-3" value={logType} onChange={setLogType}>
					{() => (
						<div>
							<ListboxButton className="relative w-full cursor-pointer rounded-lg py-1.5 pl-3 pr-10 text-left saturate-[50%] border border-zinc-700/50 hover:border-zinc-600/50 bg-zinc-800/50 text-zinc-50 hover:bg-zinc-700/50 shadow-sm focus:outline-none sm:text-sm sm:leading-6">
								<span className="block truncate">{logType.name}</span>
								<span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
									<ChevronUpDownIcon className="h-5 w-5 text-zinc-500" aria-hidden="true" />
								</span>
							</ListboxButton>
							<ListboxOptions
								transition
								className="absolute z-10 -mt-2 max-h-60 w-full overflow-auto rounded-lg bg-zinc-900/80 backdrop-blur-md border border-zinc-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none text-base p-1 text-base shadow-lg focus:outline-none data-[closed]:data-[leave]:opacity-0 data-[leave]:transition data-[leave]:duration-100 data-[leave]:ease-in sm:text-sm -translate-y-full transform">
								{logTypes.map((item) => (
									<ListboxOption
										key={item.id}
										className={({ focus }) =>
											classNames(
												focus ? 'bg-zinc-800/80 text-zinc-50' : '',
												!focus ? 'text-zinc-200' : '',
												'relative rounded-md block p-2 w-full text-left cursor-pointer select-none'
											)
										}
										value={item}>
										{({ selected, focus }) => (
											<>
												<span className={classNames(selected ? 'font-semibold' : 'font-normal', 'block truncate')}>{item.name}</span>

												{selected ? (
													<span className="text-emerald-500 absolute inset-y-0 right-0 flex items-center pr-1.5">
														<CheckIcon className="h-4 w-4" aria-hidden="true" />
													</span>
												) : null}
											</>
										)}
									</ListboxOption>
								))}
							</ListboxOptions>
						</div>
					)}
				</Listbox>
			</div>
		);
	}
};

const View = (props: { id: string; base: string }) => {
	const [item, setItem] = useState<any>();
	const [loaded, setLoaded] = useState(false);
	const [disabled, setDisabled] = useState(false);
	const [live, setLive] = useState<SSE | null>(null);
	const [liveReload, setLiveReload] = useState(false);

	const badge = {
		online: 'bg-emerald-400/10 text-emerald-400',
		stopped: 'bg-red-500/10 text-red-500',
		crashed: 'bg-amber-400/10 text-amber-400'
	};

	const server = new URLSearchParams(window.location.search).get('server') ?? 'local';

	const openConnection = () => {
		let retryTimeout;
		let hasRun = false;

		const source = new SSE(`${props.base}/live/process/${server}/${props.id}`, { headers });

		setLive(source);
		setDisabled(true);

		source.onmessage = (event) => {
			const data = JSON.parse(event.data);

			setItem(data);
			setDisabled(false);

			if (data.info.status == 'stopped') {
				source.close();
			}
			if (!hasRun) {
				setLoaded(true);
				hasRun = true;
			}
		};

		source.onerror = (error) => {
			source.close();
			retryTimeout = setTimeout(() => {
				openConnection();
			}, 5000);
		};

		return retryTimeout;
	};

	useEffect(() => {
		const retryTimeout = openConnection();

		return () => {
			live && live.close();
			clearTimeout(retryTimeout);
		};
	}, []);

	const action = (id: number, name: string) => {
		server != 'local'
			? api.post(`${props.base}/remote/${server}/action/${id}`, { json: { method: name } }).then(() => openConnection())
			: api.post(`${props.base}/process/${id}/action`, { json: { method: name } }).then(() => openConnection());
	};

	if (!loaded) {
		return <Loader />;
	} else {
		const online = isRunning(item.info.status);
		const [uptime, upunit] = startDuration(item.info.uptime);
		const [memory, memunit] = formatMemory(online ? item.stats.memory_usage.rss : 0);

		const stats = [
			{ name: 'Status', value: item.info.status },
			{ name: 'Uptime', value: online ? uptime : 'none', unit: online ? upunit : '' },
			{ name: 'Memory', value: online ? memory.toFixed(2) : 'offline', unit: online ? memunit : '' },
			{ name: 'CPU', value: online ? item.stats.cpu_percent.toFixed(2) : 'offline', unit: online ? '%' : '' }
		];

		return (
			<Fragment>
				<div className="absolute top-2 right-3 z-[200]">
					<span className="text-xs text-zinc-500 mr-2">{liveReload ? 'Fetching logs live' : 'Live logs paused'}</span>
					<span className="inline-flex items-center gap-x-1.5 rounded-md px-2 py-1 text-xs font-medium text-white ring-1 ring-inset ring-zinc-800">
						<svg viewBox="0 0 6 6" aria-hidden="true" className="h-1.5 w-1.5 fill-green-400">
							<circle r={3} cx={3} cy={3} />
						</svg>
						{server != 'local' ? server : 'Internal'}
					</span>
				</div>
				<div className="flex items-start justify-between gap-x-8 gap-y-4 bg-zinc-700/10 px-4 py-4 flex-row items-center sm:px-6 lg:px-8">
					<div>
						<div className="flex items-center gap-x-3">
							<h1 className="flex gap-x-1 text-base leading-7">
								<span className="font-semibold text-white cursor-default">{item.info.name}</span>
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
					<div className="flex lg:ml-4 mt-0">
						<span>
							<button
								type="button"
								disabled={disabled}
								onClick={() => action(props.id, 'restart')}
								className="disabled:opacity-50 disabled:pointer-events-none transition inline-flex items-center justify-center space-x-1.5 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 saturate-[110%] border-zinc-700 hover:border-zinc-600 bg-zinc-800 text-zinc-50 hover:bg-zinc-700 px-4 py-2 text-sm font-semibold rounded-lg">
								{disabled ? (
									<svg className="w-5 h-5 text-zinc-800 animate-spin fill-zinc-50" viewBox="0 0 100 101" fill="none">
										<path
											d="M100 50.5908C100 78.2051 77.6142 100.591 50 100.591C22.3858 100.591 0 78.2051 0 50.5908C0 22.9766 22.3858 0.59082 50 0.59082C77.6142 0.59082 100 22.9766 100 50.5908ZM9.08144 50.5908C9.08144 73.1895 27.4013 91.5094 50 91.5094C72.5987 91.5094 90.9186 73.1895 90.9186 50.5908C90.9186 27.9921 72.5987 9.67226 50 9.67226C27.4013 9.67226 9.08144 27.9921 9.08144 50.5908Z"
											fill="currentColor"
										/>
										<path
											d="M93.9676 39.0409C96.393 38.4038 97.8624 35.9116 97.0079 33.5539C95.2932 28.8227 92.871 24.3692 89.8167 20.348C85.8452 15.1192 80.8826 10.7238 75.2124 7.41289C69.5422 4.10194 63.2754 1.94025 56.7698 1.05124C51.7666 0.367541 46.6976 0.446843 41.7345 1.27873C39.2613 1.69328 37.813 4.19778 38.4501 6.62326C39.0873 9.04874 41.5694 10.4717 44.0505 10.1071C47.8511 9.54855 51.7191 9.52689 55.5402 10.0491C60.8642 10.7766 65.9928 12.5457 70.6331 15.2552C75.2735 17.9648 79.3347 21.5619 82.5849 25.841C84.9175 28.9121 86.7997 32.2913 88.1811 35.8758C89.083 38.2158 91.5421 39.6781 93.9676 39.0409Z"
											fill="currentFill"
										/>
									</svg>
								) : online ? (
									'Restart'
								) : (
									'Start'
								)}
							</button>
						</span>
						<span className="ml-3">
							<Menu as="div" className="relative inline-block text-left">
								<div>
									<MenuButton className="transition inline-flex items-center justify-center space-x-1.5 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 border-zinc-700 bg-transparent hover:bg-zinc-800 p-2 text-sm font-semibold rounded-lg">
										<EllipsisVerticalIcon className="h-5 w-5 text-zinc-50" aria-hidden="true" />
									</MenuButton>
								</div>

								<Transition
									as={Fragment}
									enter="transition ease-out duration-100"
									enterFrom="transform opacity-0 scale-95"
									enterTo="transform opacity-100 scale-100"
									leave="transition ease-in duration-75"
									leaveFrom="transform opacity-100 scale-100"
									leaveTo="transform opacity-0 scale-95">
									<MenuItems
										anchor={{ to: 'bottom end', gap: '8px', padding: '16px' }}
										className="z-10 w-48 origin-top-right rounded-lg bg-zinc-900/80 backdrop-blur-md border border-zinc-800 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none text-base divide-y divide-zinc-800/50">
										<div className="p-1.5">
											<MenuItem>
												{({ focus }) => (
													<a
														onClick={() => action(props.id, 'stop')}
														className={classNames(
															focus ? 'bg-yellow-400/10 text-amber-500' : 'text-zinc-200',
															'rounded-md block p-2 w-full text-left cursor-pointer'
														)}>
														Terminate
													</a>
												)}
											</MenuItem>
											<MenuItem>
												{({ focus }) => <Rename server={server} base={props.base} process_id={props.id} active={focus} old={item.info.name} />}
											</MenuItem>
											<MenuItem>
												{({ _ }) => (
													<a
														onClick={() => {
															action(props.id, 'flush');
															window.location.reload();
														}}
														className="text-zinc-200 rounded-md block p-2 w-full text-left cursor-pointer hover:bg-zinc-800/80 hover:text-zinc-50">
														Clean Logs
													</a>
												)}
											</MenuItem>
										</div>
										<div className="p-1.5">
											<MenuItem>
												{({ focus }) => (
													<a
														onClick={() => action(props.id, 'delete')}
														className={classNames(
															focus ? 'bg-red-700/10 text-red-500' : 'text-red-400',
															'rounded-md block p-2 w-full text-left cursor-pointer'
														)}>
														Delete
													</a>
												)}
											</MenuItem>
										</div>
									</MenuItems>
								</Transition>
							</Menu>
						</span>
					</div>
				</div>

				<div className="grid bg-zinc-700/10 grid-cols-4 border-b border-white/[.03] shadow-lg">
					{stats.map((stat: any, index: number) => (
						<div
							key={stat.name}
							className={classNames(
								index % 2 === 1 ? 'border-l' : index === 2 ? 'border-l' : '',
								'border-t border-white/5 py-6 px-4 sm:px-6 lg:px-8'
							)}>
							<p className="text-sm font-medium leading-6 text-zinc-400">{stat.name}</p>
							<p className="mt-2 flex items-baseline gap-x-2">
								<span className="text-xl sm:text-3xl lg:text-4xl font-semibold tracking-tight text-white">{stat.value}</span>
								{stat.unit ? <span className="text-sm text-zinc-400">{stat.unit}</span> : null}
							</p>
						</div>
					))}
				</div>

				<LogViewer server={server} id={parseInt(props.id)} base={props.base} liveReload={liveReload} setLiveReload={setLiveReload} />
			</Fragment>
		);
	}
};

export default View;
