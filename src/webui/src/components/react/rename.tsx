import { api } from '@/api';
import Modal from '@/components/react/modal';
import { useEffect, useState, Fragment } from 'react';

const Rename = (props: { base: string; server: string; process_id: number; callback: any; old: string }) => {
	const [open, setOpen] = useState(false);
	const [formData, setFormData] = useState('');

	const handleChange = (event: any) => setFormData(event.target.value);

	const handleSubmit = (event: any) => {
		const url =
			props.server != 'local'
				? `${props.base}/remote/${props.server}/rename/${props.process_id}`
				: `${props.base}/process/${props.process_id}/rename`;

		event.preventDefault();
		api.post(url, { body: formData }).then(() => {
			setOpen(false);
			props.callback();
		});
	};

	useEffect(() => {
		setFormData(props.old);
	}, []);

	return (
		<Fragment>
			<a
				onClick={() => setOpen(true)}
				className="text-zinc-200 rounded-md block p-2 w-full text-left cursor-pointer hover:bg-zinc-800/80 hover:text-zinc-50">
				Rename
			</a>
			<Modal show={open} title="Rename this process" callback={(close: boolean) => setOpen(close)}>
				<form onSubmit={handleSubmit}>
					<div className="relative border border-zinc-700 rounded-lg px-3 py-3 focus-within:ring-1 focus-within:ring-zinc-300 focus-within:border-zinc-300 sm:w-[29rem] focus-within:shadow-sm transition bg-zinc-900">
						<input
							type="text"
							name="name"
							id="name"
							value={formData}
							onChange={handleChange}
							className="bg-zinc-900 block w-full border-0 p-0 text-zinc-100 placeholder-zinc-500 focus:ring-0 sm:text-sm transition"
							placeholder={props.old}
						/>
					</div>
					<div className="bg-zinc-950 border-t border-zinc-800 px-3 py-2.5 px-6 sm:flex sm:flex-row-reverse -mb-4 mt-4 -ml-6 -mr-6">
						<button
							type="submit"
							className="mt-1.5 sm:mt-0 w-full inline-flex justify-center rounded-lg border shadow-sm px-2.5 py-2 sm:py-1 bg-sky-600 text-base font-medium text-white hover:bg-sky-500 border-sky-500 hover:border-sky-400 sm:ml-3 sm:w-auto sm:text-sm focus:outline-none transition">
							Rename
						</button>
						<button
							type="button"
							className="mt-3 w-full inline-flex justify-center rounded-lg sm:border shadow-sm px-2.5 py-1.5 sm:py-1 bg-zinc-950 text-base font-medium border-zinc-800 hover:border-zinc-700 text-zinc-50 hover:bg-zinc-800 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm focus:outline-none transition"
							onClick={() => setOpen(false)}>
							Cancel
						</button>
					</div>
				</form>
			</Modal>
		</Fragment>
	);
};

export default Rename;
