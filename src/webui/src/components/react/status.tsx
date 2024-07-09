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

	const [loaded, setLoaded] = useState(false);
	const [live, setLive] = useState<SSE | null>(null);

	const options = {
		responsive: true,
		maintainAspectRatio: false,
		animation: { duration: 0.5 },
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

		const source = new SSE(`${props.base}/live/daemon/${props.server}/metrics`, { headers });

		setLive(source);

		source.onmessage = (event) => {
			const data = JSON.parse(event.data);

			memoryUsage.pushMax(data.raw.memory_usage.rss);
			cpuPercentage.pushMax(data.raw.cpu_percent + 1);

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

	const stats = [
		{ name: 'Status', stat: 'Online' },
		{ name: 'Proccess', stat: '3' },
		{ name: 'Errors', stat: '10' },
		{ name: 'Crashes', stat: '2' }
	];

	if (!loaded) {
		return <Loader />;
	} else {
		return (
			<Fragment>
				<h3 className="ml-8 mt-6 mb-5 text-2xl font-bold leading-6 text-zinc-200">Overview</h3>
				<dl className="grid grid-cols-1 gap-5 sm:grid-cols-4 px-5">
					{stats.map((item: any) => (
						<div key={item.name} className="overflow-hidden rounded-lg bg-zinc-900/25 border border-zinc-800 px-4 py-5 shadow sm:p-6">
							<dt className="truncate text-sm font-medium text-zinc-400">{item.name}</dt>
							<dd className="mt-1 text-3xl font-semibold tracking-tight text-zinc-100">{item.stat}</dd>
						</div>
					))}
				</dl>
				<h3 className="ml-8 mt-8 mb-5 text-2xl font-bold leading-6 text-zinc-200">Metrics</h3>
				<dl className="grid grid-cols-1 gap-5 sm:grid-cols-2 px-5">
					<div className="overflow-hidden rounded-lg bg-zinc-900/25 border border-zinc-800 shadow">
						<dt className="truncate text-sm font-bold text-zinc-400 pt-4 px-4">CPU Usage</dt>
						<dt className="truncate text-xl font-bold text-zinc-100 p-1 px-4">
							{cpuPercentage.value.slice(-1)[0].toFixed(2)}
							<span className="text-base text-zinc-400">%</span>
						</dt>
						<dd className="mt-2 text-3xl font-semibold tracking-tight text-zinc-100 h-52" style={chartContainerStyle}>
							<Line data={cpuChart} options={options} />
						</dd>
					</div>
					<div className="overflow-hidden rounded-lg bg-zinc-900/25 border border-zinc-800 shadow">
						<dt className="truncate text-sm font-bold text-zinc-400 pt-4 px-4">Memory Usage</dt>
						<dt className="truncate text-xl font-bold text-zinc-100 p-1 px-4">{bytesToSize(memoryUsage.value.slice(-1)[0], 2)}</dt>
						<dd className="mt-2 text-3xl font-semibold tracking-tight text-zinc-100 h-52" style={chartContainerStyle}>
							<Line data={memoryChart} options={options} />
						</dd>
					</div>
				</dl>
			</Fragment>
		);
	}
};

export default Status;
