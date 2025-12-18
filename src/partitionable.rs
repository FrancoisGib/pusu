// use anyhow::Result;

// const PARTITION_SIZE: usize = 4;

// #[derive(Debug)]
// pub struct Partition {
//     partition_number: usize,
//     payload: [u8; PARTITION_SIZE],
//     size: usize,
// }

// pub trait Partitionable: Sized  {
//     fn partition(&self) -> Result<impl Iterator<Item = Partition>>;

//     fn reconstruct<T>(partitions: T) -> Result<Self> where T: Iterator<Item = Partition>;

//     fn reconstruct_with_size<T>(partitions: T, nb_blocks: usize) -> Result<Self> where T: Iterator<Item = Partition>;

//     fn reconstruct_from_vec(partitions: Vec<Partition>) -> Result<Self>;
// }

// impl Partitionable for String {
//     fn partition(&self) -> Result<impl Iterator<Item = Partition>> {
//         let partitions = self.as_bytes().chunks(PARTITION_SIZE).into_iter().enumerate().map(|(index, chunk)| {
//             let mut payload = [0u8; PARTITION_SIZE];
//             payload[..chunk.len()].copy_from_slice(chunk);
//             Partition { partition_number: index, payload, size: chunk.len() }
//         });
//         Ok(partitions)
//     }
    
//     fn reconstruct<T>(partitions: T) -> Result<Self>
//         where T: Iterator<Item = Partition> 
//     {
//         let mut str = String::with_capacity(PARTITION_SIZE);
//         for partition in partitions {
//             str.push_str(&String::from_utf8(partition.payload.to_vec())?);
//         }
//         Ok(str)
//     }

//     fn reconstruct_with_size<T>(partitions: T, nb_blocks: usize) -> Result<Self>
//         where T: Iterator<Item = Partition> 
//     {
//         let mut bytes = vec![0u8; nb_blocks * PARTITION_SIZE];
//         for partition in partitions {
//             let begin = partition.partition_number * PARTITION_SIZE;
//             let end = begin + partition.size;
//             bytes[begin..end].copy_from_slice(&partition.payload[..partition.size]);
//         }
//         Ok(String::from_utf8(bytes)?)
//     }

//     fn reconstruct_from_vec(partitions: Vec<Partition>) -> Result<Self> {
//         let mut bytes = vec![0u8; partitions.len() * PARTITION_SIZE];
//         for partition in partitions {
//             let begin = partition.partition_number * PARTITION_SIZE;
//             let end = begin + partition.size;
//             bytes[begin..end].copy_from_slice(&partition.payload[..partition.size]);
//         }
//         Ok(String::from_utf8(bytes)?)
//     }
// }
