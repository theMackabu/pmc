import { api } from '@/api';
import { useEffect, Fragment } from 'react';
import Loader from '@/components/react/loader';
import Header from '@/components/react/header';
import { version } from '../../../package.json';
import { useArray, classNames, isVersionTooFar, startDuration } from '@/helpers';

const getStatus = (remote: string, status: string): string => {
	const badge = {
		updated: 'bg-emerald-700/40 text-emerald-400',
		behind: 'bg-gray-700/40 text-gray-400',
		critical: 'bg-red-700/40 text-red-400'
	};

	if (remote == 'v0.0.0') {
		return badge['behind'];
	} else if (isVersionTooFar(version, remote.slice(1))) {
		return badge['behind'];
	} else if (remote == `v${version}`) {
		return badge['updated'];
	} else {
		return badge[status ?? 'critical'];
	}
};

const skeleton = {
	os: { name: '' },
	version: {
		pkg: 'v0.0.0',
		hash: 'none',
		build_date: 'none',
		target: ''
	},
	daemon: {
		pid: 'none',
		running: false,
		uptime: 0,
		process_count: 'none'
	}
};

const getServerIcon = (base: string, distro: string): string => {
	const distroList = [
		'Alpine',
		'Arch',
		'Amazon',
		'Macos',
		'Linux',
		'Fedora',
		'Debian',
		'CentOS',
		'NixOS',
		'FreeBSD',
		'OpenBSD',
		'OracleLinux',
		'Pop',
		'Raspbian',
		'Redhat',
		'Ubuntu'
	];

	const isDistroKnown = distroList.includes(distro);
	return `${base}/assets/${isDistroKnown ? distro.toLowerCase() : 'unknown'}.svg`;
};

const Index = (props: { base: string }) => {
	const items = useArray([]);

	const badge = {
		online: 'bg-emerald-400/10 text-emerald-400',
		offline: 'bg-red-500/10 text-red-500'
	};

	async function fetch() {
		items.clear();

		const metrics = await api.get(props.base + '/daemon/metrics').json();
		items.push({ ...metrics, name: 'local' });

		try {
			const servers = await api.get(props.base + '/daemon/servers').json();
			await servers.forEach(async (name) => {
				api
					.get(props.base + `/remote/${name}/metrics`)
					.json()
					.then((metrics) => items.push({ ...metrics, name }))
					.catch(() => items.push({ ...skeleton, name }));
			});
		} catch {}
	}

	useEffect(() => {
		fetch();
	}, []);

	if (items.isEmpty()) {
		return <Loader />;
	} else {
		return (
			<Fragment>
				<Header name="Servers" description="A list of all the servers in your daemon config.">
					<button
						type="button"
						onClick={fetch}
						className="transition inline-flex items-center justify-center space-x-1.5 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 border-zinc-900 hover:border-zinc-800 bg-zinc-950 text-zinc-50 hover:bg-zinc-900 px-4 py-2 text-sm font-semibold rounded-lg">
						Refresh
					</button>
				</Header>
				<table className="w-full whitespace-nowrap text-left">
					<colgroup>
						<col className="w-full sm:w-3/12" />
						<col className="lg:w-[10%]" />
						<col className="lg:w-2/12" />
						<col className="lg:w-2/12" />
						<col className="lg:w-1/12" />
						<col className="lg:w-1/12" />
						<col className="lg:w-1/12" />
						<col className="lg:w-1/12" />
					</colgroup>
					<thead className="sticky top-0 z-10 bg-zinc-950 bg-opacity-75 backdrop-blur backdrop-filter border-b border-white/10 text-sm leading-6 text-white">
						<tr>
							<th scope="col" className="py-2 pl-4 pr-8 font-semibold sm:pl-6 lg:pl-8">
								Server
							</th>
							<th scope="col" className="py-2 pl-0 pr-8 font-semibold table-cell">
								Version
							</th>
							<th scope="col" className="hidden py-2 pl-0 pr-8 font-semibold sm:table-cell">
								Build
							</th>
							<th scope="col" className="hidden py-2 pl-0 pr-8 font-semibold sm:table-cell">
								Hash
							</th>
							<th scope="col" className="hidden py-2 pl-0 pr-8 font-semibold sm:table-cell">
								Process Id
							</th>
							<th scope="col" className="hidden py-2 pl-0 pr-8 font-semibold md:table-cell lg:pr-20">
								Count
							</th>
							<th scope="col" className="hidden py-2 pl-0 pr-4 font-semibold md:table-cell lg:pr-20">
								Status
							</th>
							<th scope="col" className="py-2 pl-0 pr-4 text-right font-semibold sm:table-cell sm:pr-6 lg:pr-8">
								Uptime
							</th>
						</tr>
					</thead>
					<tbody className="divide-y divide-white/5 border-b border-white/5">
						{items.value.sort().map((server) => (
							<tr
								className={classNames(server.daemon.running && 'hover:bg-zinc-800/30 cursor-pointer', 'transition')}
								key={server.name}
								onClick={() => server.daemon.running && (window.location.href = props.base + '/status/' + server.name)}>
								<td className="py-4 pl-4 pr-8 sm:pl-6 lg:pl-8">
									<div className="flex items-center gap-x-4">
										<img
											src={getServerIcon(props.base, server.os.name)}
											className={classNames(
												server.daemon.running ? 'ring-emerald-400 bg-white' : 'ring-red-400 bg-red-500',
												'h-8 w-8 rounded-full ring-2'
											)}
										/>
										<div className="truncate text-sm font-medium leading-6 text-white">{server.name == 'local' ? 'Internal' : server.name}</div>
									</div>
								</td>
								<td className="py-4 pl-0 pr-4 table-cell sm:pr-8">
									<div className="flex gap-x-3">
										<div
											className={classNames(
												getStatus(server.version.pkg, server.version.status),
												'rounded-md px-2 py-1 text-xs font-medium ring-1 ring-inset ring-white/10'
											)}>
											{server.version.pkg}
										</div>
									</div>
								</td>
								<td className="hidden py-4 pl-0 pr-4 sm:table-cell sm:pr-8">
									<div className="font-mono text-sm leading-6 text-gray-400">
										{server.version.target} {server.version.build_date}
									</div>
								</td>
								<td className="hidden py-4 pl-0 pr-4 sm:table-cell sm:pr-8">
									<div className="font-mono text-sm leading-6 text-gray-400">{server.version.hash.slice(0, 16)}</div>
								</td>

								<td className="hidden py-4 pl-0 pr-8 text-sm leading-6 text-gray-400 md:table-cell lg:pr-20 font-mono">{server.daemon.pid}</td>
								<td className="hidden py-4 pl-0 pr-8 text-sm leading-6 text-gray-400 md:table-cell lg:pr-20">{server.daemon.process_count}</td>
								<td className="py-4 pl-0 pr-4 text-sm leading-6 sm:pr-8 lg:pr-20">
									<div className="flex items-center justify-end gap-x-2 sm:justify-start">
										<span className="text-gray-400 sm:hidden">{server.daemon.uptime == 0 ? 'none' : startDuration(server.daemon.uptime, false)}</span>
										<div className={classNames(badge[server.daemon.running ? 'online' : 'offline'], 'flex-none rounded-full p-1')}>
											<div className="h-1.5 w-1.5 rounded-full bg-current" />
										</div>
										<div className="hidden text-white sm:block">{server.daemon.running ? 'Online' : 'Offline'}</div>
									</div>
								</td>
								<td className="hidden py-4 pl-0 pr-4 text-right text-sm leading-6 text-gray-400 sm:table-cell sm:pr-6 lg:pr-8">
									{server.daemon.uptime == 0 ? 'none' : startDuration(server.daemon.uptime, false)}
								</td>
							</tr>
						))}
					</tbody>
				</table>
			</Fragment>
		);
	}
};

export default Index;
