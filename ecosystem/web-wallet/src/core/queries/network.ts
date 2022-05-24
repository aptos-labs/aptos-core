import axios from 'axios';
import { LOCAL_FAUCET_URL, LOCAL_NODE_URL } from 'core/constants';

export const getLocalhostIsLive = async () => {
  try {
    const localNode = axios.get(LOCAL_NODE_URL);
    const localFaucet = axios.get(LOCAL_FAUCET_URL);
    const localHostIsLive = await Promise.all(
      [localNode, localFaucet],
    ).then(([localNodeValue, localFaucetValue]) => (
      localNodeValue.status === 200 && localFaucetValue.status === 200
    ));
    return localHostIsLive;
  } catch (err: any) {
    // TODO, this MUST be changed in the future, currently there are CORS issues
    // on faucet and its difficult to tell if the faucet port is live. Current
    // behavior is that it just assumes its live if localFaucet returns an error.
    // Should be fixed so that CORS errors are eliminated and we can accurately
    // tell if the network is live or not
    if (err.config.url === 'http://0.0.0.0:8000') {
      return true;
    }
    return false;
  }
};

export default getLocalhostIsLive;
