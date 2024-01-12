import { $settings } from '@/store';
import { useState, Fragment } from 'react';
import favicon from '@/public/favicon.svg?url';

const Login = (props: { base: string }) => {
	const [token, setToken] = useState('');
	const [loginFailed, setLoginFailed] = useState(false);

	const handleChange = (event: any) => setToken(event.target.value);

	const handleSubmit = (event: any) => {
		event.preventDefault();
		$settings.setKey('token', token);

		fetch(props.base + '/daemon/metrics', {
			headers: { token },
		}).then((response) => {
			if (response.status === 200) {
				window.location.href = props.base;
			} else {
				setLoginFailed(true);
				setTimeout(() => {
					setLoginFailed(false);
				}, 3000);
			}
		});
	};

	return (
		<Fragment>
			{loginFailed && (
				<div className="-mb-[92px] sm:mx-auto sm:w-full sm:max-w-[480px] sm:rounded-lg bg-red-600 sm:border border-red-400/50 p-4 sm:mt-4 sm:-mb-[110px]">
					<h3 className="text-sm font-medium text-white">Failed to login. Is the token correct?</h3>
				</div>
			)}
			<div className="h-screen flex items-center -mt-10">
				<div className="flex min-h-full flex-1 flex-col justify-center px-0 sm:px-6 lg:px-8">
					<div className="sm:mx-auto sm:w-full sm:max-w-md">
						<img className="mx-auto h-10 w-auto" src={`${props.base}${favicon}`} alt="PMC" />
						<h2 className="mt-6 text-center text-2xl font-bold leading-9 tracking-tight text-zinc-200">Provide token to continue</h2>
					</div>

					<div className="mt-10 sm:mx-auto sm:w-full sm:max-w-[480px]">
						<div className="px-5 py-6 rounded-lg bg-none sm:border border-zinc-700/30 sm:bg-zinc-900/10">
							<form className="space-y-6" onSubmit={handleSubmit}>
								<div>
									<div>
										<input
											required
											id="password"
											name="password"
											type="password"
											value={token}
											onChange={handleChange}
											placeholder="••••••••••••••••"
											autoComplete="current-password"
											className="block w-full rounded-md border-0 bg-white/5 py-2 text-white shadow-sm ring-1 ring-inset ring-white/10 focus:ring-2 focus:ring-inset focus:ring-sky-500 sm:text-sm sm:leading-6 placeholder:text-zinc-600"
										/>
									</div>
								</div>

								<div>
									<button
										type="submit"
										className="-mb-1 transition flex w-full justify-center rounded-md px-3 py-1.5 text-sm font-semibold leading-6 text-white shadow-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 border focus:outline-none focus:ring-0 focus:ring-offset-0 focus:z-10 shrink-0 saturate-[110%] border-zinc-700 hover:border-zinc-600 bg-zinc-800 text-zinc-50 hover:bg-zinc-700">
										Continue
									</button>
								</div>
							</form>
						</div>
					</div>
				</div>
			</div>
		</Fragment>
	);
};

export default Login;
