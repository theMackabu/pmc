import { Line } from 'react-chartjs-2';
import { SSE, api, headers } from '@/api';
import Loader from '@/components/react/loader';
import { useEffect, useState, useRef, Fragment } from 'react';
import { classNames, isRunning, formatMemory, startDuration, useArray } from '@/helpers';
import { Chart, CategoryScale, LinearScale, PointElement, LineElement, Filler } from 'chart.js';

Chart.register(CategoryScale, LinearScale, PointElement, LineElement, Filler);

const bytesToSize = (bytes: number, precision: number) => {
	if (isNaN(bytes) || bytes === 0) return '0b';

	const sizes = ['b', 'kb', 'mb', 'gb', 'tb'];
	const kilobyte = 1024;

	const index = Math.floor(Math.log(bytes) / Math.log(kilobyte));
	const size = (bytes / Math.pow(kilobyte, index)).toFixed(precision);
	return size + sizes[index];
};

const Status = (props: { name: string; base: string }) => {
	const bufferLength = 21;
	const memoryUsage = useArray([], bufferLength);
	const cpuPercentage = useArray([], bufferLength);

	const [item, setItem] = useState<any>();
	const [loaded, setLoaded] = useState(false);
	const [live, setLive] = useState<SSE | null>(null);

	const options = {
		responsive: true,
		maintainAspectRatio: false,
		animation: { duration: 0 },
		layout: {
			padding: {
				left: 0,
				right: 0,
				bottom: 0,
				top: 0
			}
		},
		plugins: {
			tooltips: { enabled: false },
			title: { display: false }
		},
		elements: {
			point: { radius: 0 },
			line: { tension: 0.5, borderWidth: 1 }
		},
		scales: {
			x: { display: false },
			y: { display: false, suggestedMin: 0 }
		},
		data: {
			labels: Array(20).fill(''),
			datasets: [{ fill: true, data: Array(20).fill(0) }]
		}
	};

	const chartContainerStyle = {
		borderRadius: '0 0 0.5rem 0.5rem',
		marginBottom: '0.5px',
		zIndex: 1
	};

	const cpuChart = {
		labels: Array(20).fill(''),
		datasets: [
			{
				fill: true,
				data: cpuPercentage.value,
				borderColor: '#0284c7',
				backgroundColor: (ctx: any) => {
					const chart = ctx.chart;
					const { ctx: context, chartArea } = chart;
					if (!chartArea) {
						return null;
					}

					const gradient = context.createLinearGradient(0, chartArea.bottom, 0, chartArea.top);
					gradient.addColorStop(0, 'rgba(14, 165, 233, 0.1)');
					gradient.addColorStop(1, 'rgba(14, 165, 233, 0.5)');

					return gradient;
				}
			}
		]
	};

	const memoryChart = {
		labels: Array(20).fill(''),
		datasets: [
			{
				fill: true,
				data: memoryUsage.value,
				borderColor: '#0284c7',
				backgroundColor: (ctx: any) => {
					const chart = ctx.chart;
					const { ctx: context, chartArea } = chart;
					if (!chartArea) {
						return null;
					}

					const gradient = context.createLinearGradient(0, chartArea.bottom, 0, chartArea.top);
					gradient.addColorStop(0, 'rgba(14, 165, 233, 0.1)');
					gradient.addColorStop(1, 'rgba(14, 165, 233, 0.5)');

					return gradient;
				}
			}
		]
	};

	const openConnection = () => {
		let retryTimeout;
		let hasRun = false;

		const source = new SSE(`${props.base}/live/daemon/${props.name}/metrics`, { headers });

		setLive(source);

		source.onmessage = (event) => {
			const data = JSON.parse(event.data);

			setItem(data);

			memoryUsage.pushMax(data.raw.memory_usage);
			cpuPercentage.pushMax(data.raw.cpu_percent);

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

	if (!loaded) {
		return <Loader />;
	} else {
		const stats = [
			{ name: 'Uptime', stat: startDuration(item.daemon.uptime, false) },
			{ name: 'Count', stat: item.daemon.process_count },
			{ name: 'Version', stat: item.version.pkg },
			{ name: 'Process Id', stat: item.daemon.pid },
			{ name: 'Build date', stat: item.version.build_date },
			{ name: 'Hash', stat: item.version.hash.slice(0, 18) },
			{ name: 'Platform', stat: `${item.os.name} ${item.os.version} (${item.os.arch})` },
			{ name: 'Daemon', stat: item.daemon.daemon_type }
		];

		return (
			<Fragment>
				<div className="absolute top-2 right-3 z-[200]">
					<span className="inline-flex items-center gap-x-1.5 rounded-md px-2 py-1 text-xs font-medium text-white ring-1 ring-inset ring-zinc-800">
						<svg viewBox="0 0 6 6" aria-hidden="true" className="h-1.5 w-1.5 fill-green-400">
							<circle r={3} cx={3} cy={3} />
						</svg>
						{props.name != 'local' ? props.name : 'Internal'}
					</span>
				</div>
				<dl className="mt-8 grid grid-cols-1 gap-5 sm:grid-cols-2 px-5">
					<div className="overflow-hidden rounded-lg bg-zinc-900/20 border border-zinc-800 shadow">
						<dt className="truncate text-sm font-bold text-zinc-400 pt-4 px-4">CPU Usage</dt>
						<dt className="truncate text-xl font-bold text-zinc-100 p-1 px-4">
							{cpuPercentage.value.slice(-1)[0].toFixed(2)}
							<span className="text-base text-zinc-400">%</span>
						</dt>
						<dd className="mt-2 text-3xl font-semibold tracking-tight text-zinc-100 h-96" style={chartContainerStyle}>
							<Line data={cpuChart} options={options} />
						</dd>
					</div>
					<div className="overflow-hidden rounded-lg bg-zinc-900/20 border border-zinc-800 shadow">
						<dt className="truncate text-sm font-bold text-zinc-400 pt-4 px-4">Memory Usage</dt>
						<dt className="truncate text-xl font-bold text-zinc-100 p-1 px-4">{bytesToSize(memoryUsage.value.slice(-1)[0], 2)}</dt>
						<dd className="mt-2 text-3xl font-semibold tracking-tight text-zinc-100 h-96" style={chartContainerStyle}>
							<Line data={memoryChart} options={options} />
						</dd>
					</div>
				</dl>
				<dl className="mt-5 pb-5 grid grid-cols-2 gap-5 lg:grid-cols-4 px-5 h-3/10">
					{stats.map((item: any) => (
						<div key={item.name} className="overflow-hidden rounded-lg bg-zinc-900/20 border border-zinc-800 px-4 py-5 shadow sm:p-6">
							<dt className="truncate text-sm font-medium text-zinc-400">{item.name}</dt>
							<dd className="mt-1 text-2xl font-semibold tracking-tight text-zinc-100">{item.stat}</dd>
						</div>
					))}
				</dl>
			</Fragment>
		);
	}
};

export default Status;
