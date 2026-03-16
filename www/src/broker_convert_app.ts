import { AcbAppRunMode } from './common/acb_app_types.js';
import { ErrorBox } from './vue/error_box_store.js';

export function runHandler(_mode: AcbAppRunMode): void {
   ErrorBox.getBrokerConvert().showWith({
      title: 'Not Implemented',
      descPre: 'Broker Activity Convert is not yet implemented.',
   });
}
