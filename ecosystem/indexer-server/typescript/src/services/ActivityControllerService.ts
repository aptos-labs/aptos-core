/* eslint-disable no-async-promise-executor */
/* eslint-disable no-unused-vars */
import Service from './Service';

interface GetActivitiesByUserParams {
  from?: Date;
  size?: number;
  sort?: any;
  to?: Date;
  type: any;
  user: string[]
}

/**
* Returns activities by user
*
* type List Activity type
* user List Addresses of the users
* from Date Lower time border of data (optional)
* to Date Upper time border of data (optional)
* size Integer The number of items to return (optional)
* sort ActivitySort Sorting by data update time (optional)
* returns Activities
* */
const getActivitiesByUser = ({
  from, size, sort, to, type, user,
}: GetActivitiesByUserParams) => new Promise(
  async (resolve, reject) => {
    try {
      resolve(Service.successResponse({
        from,
        size,
        sort,
        to,
        type,
        user,
      }));
    } catch (e: any) {
      reject(Service.rejectResponse(
        e.message || 'Invalid input',
        e.status || 405,
      ));
    }
  },
);

export default {
  getActivitiesByUser,
};
